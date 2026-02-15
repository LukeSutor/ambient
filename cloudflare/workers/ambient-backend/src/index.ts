import { GenerateContentConfig, GoogleGenAI, ThinkingLevel } from "@google/genai";
import { createClient, SupabaseClient } from '@supabase/supabase-js'

interface Env {
	SUPABASE_URL: string;
	SUPABASE_ANON_KEY: string;
	SUPABASE_SERVICE_ROLE_KEY: string;
	GEMINI_API_KEY: string;
	GOOGLE_CLIENT_ID: string;
	GOOGLE_CLIENT_SECRET: string;
}

type requestData = {
	modelType: string;
	content: Array<{ role: string; parts: object[] }>;
	stream: boolean;
	systemPrompt?: string;
	jsonSchema?: object;
	tools?: any;
	/** Session token from /v1/usage/start-turn — bypasses per-call rate limiting */
	sessionToken?: string;
}

// Map model keys to Gemini API model names, thinking config, and credit cost.
// Credit costs are per-request. Supports decimals for future cheaper models.
const MODEL_REGISTRY: Record<string, { apiModel: string; thinkingLevel: ThinkingLevel; creditCost: number }> = {
	"gemini-3-flash": { apiModel: "gemini-3-flash-preview", thinkingLevel: ThinkingLevel.MINIMAL, creditCost: 1 },
	"gemini-3-pro": { apiModel: "gemini-3-pro-preview", thinkingLevel: ThinkingLevel.LOW, creditCost: 3 },
};

// Daily credit limits per user tier.
// -1 = unlimited. Premium is placeholder for future use.
const CREDIT_LIMITS: Record<UserTier, number> = {
	free: 10,
	premium: 50,
	admin: -1,
};

const extractModelName = (modelType: string): string | null => {
	return MODEL_REGISTRY[modelType]?.apiModel ?? null;
};

// ---------------------------------------------------------------------------
// User tier resolution
// ---------------------------------------------------------------------------

type UserTier = "free" | "premium" | "admin";

/**
 * Resolve a user's effective tier by checking their profile role
 * and (for premium) verifying an active subscription exists.
 *
 * - admin: unlimited access, set manually in Supabase
 * - premium: requires an active/trialing/past_due subscription
 * - free: default for everyone else
 *
 * Profile + subscription are fetched in parallel to avoid a
 * sequential round-trip for premium users.
 */
async function getUserTier(admin: SupabaseClient, userId: string): Promise<UserTier> {
	// Fetch profile and subscription check in parallel
	const [profileResult, subResult] = await Promise.all([
		admin.from('profiles').select('role').eq('id', userId).maybeSingle(),
		admin.from('subscriptions').select('status').eq('user_id', userId)
			.in('status', ['active', 'trialing', 'past_due']).maybeSingle(),
	]);

	if (profileResult.error || !profileResult.data) {
		console.error('Failed to fetch user profile:', profileResult.error);
		return 'free';
	}

	const role = profileResult.data.role;
	if (role === 'admin') return 'admin';
	if (role === 'premium' && subResult.data) return 'premium';
	return 'free';
}

// ---------------------------------------------------------------------------
// Credit usage (database-driven)
// ---------------------------------------------------------------------------

/** Create a Supabase admin client (service role — bypasses RLS) */
function createAdminClient(env: Env): SupabaseClient {
	return createClient(env.SUPABASE_URL, env.SUPABASE_SERVICE_ROLE_KEY);
}

/** Get today's date in YYYY-MM-DD format (UTC) */
function getTodayUTC(): string {
	return new Date().toISOString().split('T')[0];
}

/**
 * Fetch today's total credit usage for a user.
 * Returns 0 if no usage record exists yet.
 */
async function getCreditsUsedToday(admin: SupabaseClient, userId: string): Promise<number> {
	const today = getTodayUTC();
	const { data, error } = await admin
		.from('credit_usage')
		.select('credits_used')
		.eq('user_id', userId)
		.eq('usage_date', today)
		.maybeSingle();

	if (error) {
		console.error('Failed to fetch credit usage:', error);
		return 0;
	}

	return data?.credits_used ?? 0;
}

/**
 * Increment the credit usage for a user today.
 *
 * Uses Supabase's `.upsert()` with `onConflict` for a single-query operation.
 * Requires a UNIQUE constraint on (user_id, usage_date).
 */
async function incrementCreditUsage(
	admin: SupabaseClient,
	userId: string,
	currentUsed: number,
	creditCost: number,
): Promise<void> {
	const today = getTodayUTC();

	const { error } = await admin
		.from('credit_usage')
		.upsert(
			{
				user_id: userId,
				usage_date: today,
				credits_used: currentUsed + creditCost,
			},
			{ onConflict: 'user_id,usage_date' },
		);

	if (error) {
		console.error('Failed to increment credit usage:', error);
		throw new Error('Failed to increment credit usage');
	}
}

export default {
	async fetch(request, env, ctx): Promise<Response> {
		const url = new URL(request.url);

		// Handle OPTIONS for CORS
		if (request.method === 'OPTIONS') {
			return new Response(null, {
				headers: {
					'Access-Control-Allow-Origin': '*',
					'Access-Control-Allow-Methods': 'POST, OPTIONS',
					'Access-Control-Allow-Headers': 'Content-Type, Authorization',
				},
			});
		}

		// LLM Generation Proxy
		if (url.pathname === '/v1/llm/generate' || (url.pathname === '/' && request.method === 'POST')) {
			return handleLlmGenerate(request, env, ctx);
		}

		// Start a generation turn — checks rate limit, increments usage, returns session token
		if (url.pathname === '/v1/usage/start-turn' && request.method === 'POST') {
			return handleStartTurn(request, env);
		}

		// Usage/remaining endpoint — returns remaining daily uses per model
		if (url.pathname === '/v1/usage/remaining' && request.method === 'GET') {
			return handleGetRemainingUses(request, env);
		}

		// Google OAuth Refresh Proxy
		if (url.pathname === '/v1/auth/google/refresh' && request.method === 'POST') {
			return handleGoogleRefresh(request, env);
		}

		return new Response('Not Found', { status: 404 });
	},
} satisfies ExportedHandler<Env>;

/**
 * Verify Supabase user session
 */
async function verifySupabaseUser(request: Request, env: Env) {
	const authHeader = request.headers.get('Authorization');
	const token = authHeader?.startsWith('Bearer ') ? authHeader.substring(7) : null;

	if (!token) return null;

	const supabase = createClient(env.SUPABASE_URL, env.SUPABASE_ANON_KEY);
	const { data: { user } } = await supabase.auth.getUser(token);
	return user;
}

/**
 * Handle Google OAuth refresh
 */
async function handleGoogleRefresh(request: Request, env: Env): Promise<Response> {
	const user = await verifySupabaseUser(request, env);
	if (!user) {
		return new Response('Unauthorized: Invalid Supabase token', { status: 401 });
	}

	let body: { refresh_token: string };
	try {
		body = await request.json();
	} catch (e) {
		return new Response('Bad Request: Invalid JSON', { status: 400 });
	}

	if (!body.refresh_token) {
		return new Response('Bad Request: Missing refresh_token', { status: 400 });
	}

	// Call Google OAuth API
	const googleResponse = await fetch('https://oauth2.googleapis.com/token', {
		method: 'POST',
		headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
		body: new URLSearchParams({
			client_id: env.GOOGLE_CLIENT_ID,
			client_secret: env.GOOGLE_CLIENT_SECRET,
			refresh_token: body.refresh_token,
			grant_type: 'refresh_token',
		}),
	});

	const data = await googleResponse.json();
	return new Response(JSON.stringify(data), {
		status: googleResponse.status,
		headers: { 
			'Content-Type': 'application/json',
			'Access-Control-Allow-Origin': '*',
		},
	});
}

/**
 * Handle remaining credit usage query.
 *
 * Returns the user's effective tier, global credit usage, and per-model
 * credit costs. All models are available to all users — no tier-gating.
 */
async function handleGetRemainingUses(request: Request, env: Env): Promise<Response> {
	const user = await verifySupabaseUser(request, env);
	if (!user) {
		return new Response('Unauthorized: Invalid token', { status: 401 });
	}

	const admin = createAdminClient(env);

	// Resolve tier and fetch credit usage in parallel
	const [tier, creditsUsed] = await Promise.all([
		getUserTier(admin, user.id),
		getCreditsUsedToday(admin, user.id),
	]);

	const dailyCreditLimit = CREDIT_LIMITS[tier] ?? CREDIT_LIMITS.free;
	const isUnlimited = dailyCreditLimit === -1;

	// Build per-model credit costs from the registry
	const modelCosts: Record<string, number> = {};
	for (const [modelKey, info] of Object.entries(MODEL_REGISTRY)) {
		modelCosts[modelKey] = info.creditCost;
	}

	return new Response(JSON.stringify({
		user_tier: tier,
		daily_credit_limit: dailyCreditLimit,
		credits_used: isUnlimited ? 0 : creditsUsed,
		credits_remaining: isUnlimited ? -1 : Math.max(0, dailyCreditLimit - creditsUsed),
		model_costs: modelCosts,
	}), {
		headers: {
			'Content-Type': 'application/json',
			'Access-Control-Allow-Origin': '*',
		},
	});
}

/**
 * Handle starting a generation turn.
 *
 * This is called once per user turn (before the agentic loop starts).
 * It checks the global credit limit, increments usage by the model's
 * credit cost, and returns a session_token. Subsequent LLM calls in
 * the same turn include the session_token to bypass per-call limiting.
 */
async function handleStartTurn(request: Request, env: Env): Promise<Response> {
	const user = await verifySupabaseUser(request, env);
	if (!user) {
		return new Response('Unauthorized: Invalid token', { status: 401 });
	}

	let body: { modelType: string };
	try {
		body = await request.json();
	} catch {
		return new Response('Bad Request: Invalid JSON', { status: 400 });
	}

	if (!body.modelType) {
		return new Response('Bad Request: Missing modelType', { status: 400 });
	}

	// Validate model exists in registry
	const modelInfo = MODEL_REGISTRY[body.modelType];
	if (!modelInfo) {
		return new Response('Bad Request: Invalid model type', { status: 400 });
	}

	const admin = createAdminClient(env);

	// Fetch tier and credit usage in parallel
	const [tier, creditsUsed] = await Promise.all([
		getUserTier(admin, user.id),
		getCreditsUsedToday(admin, user.id),
	]);

	const dailyCreditLimit = CREDIT_LIMITS[tier] ?? CREDIT_LIMITS.free;

	// Check credit limit (skip for unlimited tiers where limit = -1)
	if (dailyCreditLimit >= 0 && creditsUsed + modelInfo.creditCost > dailyCreditLimit) {
		return new Response(JSON.stringify({
			error: 'rate_limit_exceeded',
			message: `Not enough credits. This model costs ${modelInfo.creditCost} credit(s) but you only have ${Math.max(0, dailyCreditLimit - creditsUsed)} remaining. Resets at midnight UTC.`,
			daily_credit_limit: dailyCreditLimit,
			credits_used: creditsUsed,
			credit_cost: modelInfo.creditCost,
		}), {
			status: 429,
			headers: { 'Content-Type': 'application/json', 'Access-Control-Allow-Origin': '*' },
		});
	}

	// Increment credit usage for this turn
	if (dailyCreditLimit >= 0) {
		await incrementCreditUsage(admin, user.id, creditsUsed, modelInfo.creditCost);
	}

	// Create a generation session
	const { data: session, error: sessionError } = await admin
		.from('generation_sessions')
		.insert({
			user_id: user.id,
			model_type: body.modelType,
		})
		.select('session_token, max_calls, expires_at')
		.single();

	if (sessionError || !session) {
		console.error('Failed to create generation session:', sessionError);
		return new Response('Internal Server Error: Failed to create session', { status: 500 });
	}

	return new Response(JSON.stringify({
		session_token: session.session_token,
		max_calls: session.max_calls,
		expires_at: session.expires_at,
		credit_cost: modelInfo.creditCost,
	}), {
		headers: { 'Content-Type': 'application/json', 'Access-Control-Allow-Origin': '*' },
	});
}

/**
 * Validate a session token for a generation request.
 * Returns the session data if valid, or null if invalid/expired/exhausted.
 */
async function validateSession(
	admin: SupabaseClient,
	sessionToken: string,
	userId: string,
	modelType: string,
): Promise<{ valid: boolean; reason?: string }> {
	const { data: session, error } = await admin
		.from('generation_sessions')
		.select('id, user_id, model_type, call_count, max_calls, expires_at')
		.eq('session_token', sessionToken)
		.single();

	if (error || !session) {
		return { valid: false, reason: 'Invalid session token' };
	}

	// Verify ownership
	if (session.user_id !== userId) {
		return { valid: false, reason: 'Session does not belong to this user' };
	}

	// Verify model match
	if (session.model_type !== modelType) {
		return { valid: false, reason: 'Session model mismatch' };
	}

	// Check expiry
	if (new Date(session.expires_at) < new Date()) {
		return { valid: false, reason: 'Session expired' };
	}

	// Check call count
	if (session.call_count >= session.max_calls) {
		return { valid: false, reason: 'Session call limit reached' };
	}

	// Increment call count
	await admin
		.from('generation_sessions')
		.update({ call_count: session.call_count + 1 })
		.eq('id', session.id);

	return { valid: true };
}

/**
 * Handle LLM Generation (Gemini)
 */
async function handleLlmGenerate(request: Request, env: Env, ctx: ExecutionContext): Promise<Response> {
	// Only accept POST requests
	if (request.method !== 'POST') {
		return new Response('Method Not Allowed', { status: 405 });
	}

	// Get request parameters
	let body: requestData;
	try {
			body = await request.json();
	} catch (e) {
			return new Response('Bad Request: Invalid JSON', { status: 400 });
	}

	const user = await verifySupabaseUser(request, env);
	if (!user) {
		return new Response('Unauthorized: Invalid token', { status: 401 });
	}

	// Map model type to model name
	const modelName = extractModelName(body.modelType);
	if (!modelName) {
		return new Response('Bad Request: Invalid model type', { status: 400 });
	}

	const admin = createAdminClient(env);

	// --- Session-based or legacy rate limiting ---
	// If a session token is provided, validate it (no usage increment).
	// Otherwise, fall back to per-call credit-based rate limiting.
	let creditsUsedToday = 0;

	if (body.sessionToken) {
		const sessionResult = await validateSession(admin, body.sessionToken, user.id, body.modelType);
		if (!sessionResult.valid) {
			return new Response(JSON.stringify({
				error: 'session_invalid',
				message: sessionResult.reason || 'Invalid session',
			}), {
				status: 403,
				headers: { 'Content-Type': 'application/json', 'Access-Control-Allow-Origin': '*' },
			});
		}
		// Session is valid — skip rate limiting and usage increment below
	} else {
		// Legacy path: per-call credit-based rate limiting (no session token)
		const modelInfo = MODEL_REGISTRY[body.modelType];
		const creditCost = modelInfo?.creditCost ?? 1;

		const [tier, fetchedCredits] = await Promise.all([
			getUserTier(admin, user.id),
			getCreditsUsedToday(admin, user.id),
		]);
		creditsUsedToday = fetchedCredits;

		const dailyCreditLimit = CREDIT_LIMITS[tier] ?? CREDIT_LIMITS.free;

		if (dailyCreditLimit >= 0 && creditsUsedToday + creditCost > dailyCreditLimit) {
			return new Response(JSON.stringify({
				error: 'rate_limit_exceeded',
				message: `Not enough credits. This model costs ${creditCost} credit(s) but you only have ${Math.max(0, dailyCreditLimit - creditsUsedToday)} remaining. Resets at midnight UTC.`,
				daily_credit_limit: dailyCreditLimit,
				credits_used: creditsUsedToday,
				credit_cost: creditCost,
			}), {
				status: 429,
				headers: { 'Content-Type': 'application/json', 'Access-Control-Allow-Origin': '*' },
			});
		}
	}

	// Build chat config
	let chatConfig: GenerateContentConfig = {};
	if (body.jsonSchema) {
		let schema = body.jsonSchema;
		if (typeof schema === 'string') {
			try {
				schema = JSON.parse(schema);
			} catch (e) {
				return new Response('Bad Request: Invalid JSON in jsonSchema', { status: 400 });
			}
		}
		chatConfig.responseJsonSchema = schema as object;
		chatConfig.responseMimeType = "application/json";
	}
	chatConfig.systemInstruction = body.systemPrompt || "You are a helpful assistant.";
	
	if (body.tools) {
		chatConfig.tools = [body.tools];
	}

	// Configure thinking level from the model registry
	const modelInfo = MODEL_REGISTRY[body.modelType];
	if (modelInfo) {
		chatConfig.thinkingConfig = { thinkingLevel: modelInfo.thinkingLevel };
	}		

	const ai = new GoogleGenAI({ apiKey: env.GEMINI_API_KEY });
	if (body.stream) {
		const result = await ai.models.generateContentStream({
			model: modelName,
			contents: body.content,
			config: chatConfig
		});

		const { readable, writable } = new TransformStream();
		const writer = writable.getWriter();
		const encoder = new TextEncoder();

		// Only increment credit usage for legacy (non-session) calls
		if (!body.sessionToken) {
			const modelInfo = MODEL_REGISTRY[body.modelType];
			const creditCost = modelInfo?.creditCost ?? 1;
			ctx.waitUntil(incrementCreditUsage(admin, user.id, creditsUsedToday, creditCost).catch(e =>
				console.error('Failed to increment credit usage:', e)
			));
		}

		ctx.waitUntil((async () => {
			try {
				for await (const chunk of result) {
					await writer.write(encoder.encode(`data: ${JSON.stringify(chunk)}\n\n`));
				}
				await writer.write(encoder.encode('data: [DONE]\n\n'));
			} catch (e) {
				console.error("Streaming error:", e);
				const errorMsg = e instanceof Error ? e.message : String(e);
				await writer.write(encoder.encode(`event: error\ndata: ${JSON.stringify({ error: errorMsg })}\n\n`));
			} finally {
				await writer.close();
			}
		})());

		return new Response(readable, {
			headers: {
				'Content-Type': 'text/event-stream',
				'Cache-Control': 'no-cache',
				'Connection': 'keep-alive',
				'Access-Control-Allow-Origin': '*',
			},
		});
	} else {
		const response = await ai.models.generateContent({
			model: modelName,
			contents: body.content,
			config: chatConfig
		});

		// Only increment credit usage for legacy (non-session) calls
		if (!body.sessionToken) {
			const modelInfo = MODEL_REGISTRY[body.modelType];
			const creditCost = modelInfo?.creditCost ?? 1;
			ctx.waitUntil(incrementCreditUsage(admin, user.id, creditsUsedToday, creditCost).catch(e =>
				console.error('Failed to increment credit usage:', e)
			));
		}

		return new Response(JSON.stringify(response), {
			headers: { 
				'Content-Type': 'application/json',
				'Access-Control-Allow-Origin': '*',
			},
		});
	}
}
