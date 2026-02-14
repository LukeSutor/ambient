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

// Map model keys to Gemini API model names and thinking config.
// This is the only hardcoded model info — limits come from the `model_limits` table.
const MODEL_REGISTRY: Record<string, { apiModel: string; thinkingLevel: ThinkingLevel }> = {
	"gemini-3-flash": { apiModel: "gemini-3-flash-preview", thinkingLevel: ThinkingLevel.MINIMAL },
	"gemini-3-pro": { apiModel: "gemini-3-pro-preview", thinkingLevel: ThinkingLevel.LOW },
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
// Model limits (database-driven)
// ---------------------------------------------------------------------------

interface ModelLimit {
	model_type: string;
	daily_limit: number;
	is_available: boolean;
}

/**
 * Fetch model limits for a given tier from the `model_limits` table.
 * Returns a map of model_type → { daily_limit, is_available }.
 */
async function getModelLimitsForTier(
	admin: SupabaseClient,
	tier: UserTier,
): Promise<Record<string, ModelLimit>> {
	const { data, error } = await admin
		.from('model_limits')
		.select('model_type, daily_limit, is_available')
		.eq('tier', tier);

	if (error || !data) {
		console.error('Failed to fetch model limits:', error);
		return {};
	}

	const limits: Record<string, ModelLimit> = {};
	for (const row of data) {
		limits[row.model_type] = {
			model_type: row.model_type,
			daily_limit: row.daily_limit,
			is_available: row.is_available,
		};
	}
	return limits;
}

/**
 * Fetch the limit for a single model on a given tier.
 * More efficient than getModelLimitsForTier when only one model is needed
 * (e.g. in the generate path).
 */
async function getModelLimitForTier(
	admin: SupabaseClient,
	tier: UserTier,
	modelType: string,
): Promise<ModelLimit | null> {
	const { data, error } = await admin
		.from('model_limits')
		.select('model_type, daily_limit, is_available')
		.eq('tier', tier)
		.eq('model_type', modelType)
		.maybeSingle();

	if (error || !data) return null;
	return {
		model_type: data.model_type,
		daily_limit: data.daily_limit,
		is_available: data.is_available,
	};
}

/** Get today's date in YYYY-MM-DD format (UTC) */
function getTodayUTC(): string {
	return new Date().toISOString().split('T')[0];
}

/** Create a Supabase admin client (service role — bypasses RLS) */
function createAdminClient(env: Env): SupabaseClient {
	return createClient(env.SUPABASE_URL, env.SUPABASE_SERVICE_ROLE_KEY);
}

/**
 * Fetch today's usage for ALL models for a given user in a single query.
 * Returns a map of model_type → requests_used.
 */
async function getAllUsageToday(admin: SupabaseClient, userId: string): Promise<Record<string, number>> {
	const today = getTodayUTC();
	const { data, error } = await admin
		.from('model_usage')
		.select('model_type, requests_used')
		.eq('user_id', userId)
		.eq('usage_date', today);

	if (error) {
		console.error('Failed to fetch usage:', error);
		return {};
	}

	const usage: Record<string, number> = {};
	for (const row of data ?? []) {
		usage[row.model_type] = row.requests_used;
	}
	return usage;
}

/**
 * Increment the request count for a user/model/today.
 *
 * Uses Supabase's `.upsert()` with `onConflict` for a single-query operation.
 * The caller passes in `currentUsed` (already fetched during the rate-limit
 * check), so we don't need a separate SELECT — just upsert the new count.
 *
 * Requires a UNIQUE constraint on (user_id, model_type, usage_date).
 */
async function incrementUsage(
	admin: SupabaseClient,
	userId: string,
	modelType: string,
	currentUsed: number,
): Promise<void> {
	const today = getTodayUTC();

	const { error } = await admin
		.from('model_usage')
		.upsert(
			{
				user_id: userId,
				model_type: modelType,
				usage_date: today,
				requests_used: currentUsed + 1,
			},
			{ onConflict: 'user_id,model_type,usage_date' },
		);

	if (error) {
		console.error('Failed to increment usage:', error);
		throw new Error('Failed to increment usage');
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
 * Handle remaining uses query.
 *
 * Returns the user's effective tier and per-model usage info,
 * all driven by the `profiles`, `subscriptions`, `model_limits`,
 * and `model_usage` tables.
 */
async function handleGetRemainingUses(request: Request, env: Env): Promise<Response> {
	const user = await verifySupabaseUser(request, env);
	if (!user) {
		return new Response('Unauthorized: Invalid token', { status: 401 });
	}

	const admin = createAdminClient(env);

	// Resolve tier and fetch all usage in parallel — these are independent.
	// Saves a full round-trip vs sequential tier → usage.
	const [tier, usageToday] = await Promise.all([
		getUserTier(admin, user.id),
		getAllUsageToday(admin, user.id),
	]);

	// Get limits (depends on resolved tier)
	const limits = await getModelLimitsForTier(admin, tier);

	// Build response by combining limits + batch usage (all in-memory, no queries)
	const models: Record<string, {
		daily_limit: number;
		requests_used: number;
		remaining: number;
		is_available: boolean;
	}> = {};

	for (const [modelType, limit] of Object.entries(limits)) {
		if (limit.daily_limit === -1) {
			models[modelType] = {
				daily_limit: -1,
				requests_used: 0,
				remaining: -1,
				is_available: limit.is_available,
			};
		} else {
			const used = usageToday[modelType] ?? 0;
			models[modelType] = {
				daily_limit: limit.daily_limit,
				requests_used: used,
				remaining: Math.max(0, limit.daily_limit - used),
				is_available: limit.is_available,
			};
		}
	}

	return new Response(JSON.stringify({ user_tier: tier, models }), {
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
 * It checks the rate limit, increments usage, and returns a session_token.
 * Subsequent LLM calls in the same turn include the session_token to
 * bypass per-call rate limiting.
 *
 * The session is stored in the `generation_sessions` table and is
 * server-side enforced — the client cannot forge or extend sessions.
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
	if (!MODEL_REGISTRY[body.modelType]) {
		return new Response('Bad Request: Invalid model type', { status: 400 });
	}

	const admin = createAdminClient(env);

	// Fetch tier and usage in parallel
	const [tier, usageToday] = await Promise.all([
		getUserTier(admin, user.id),
		getAllUsageToday(admin, user.id),
	]);

	const modelLimit = await getModelLimitForTier(admin, tier, body.modelType);

	// Block if model is not available for this tier
	if (!modelLimit || !modelLimit.is_available) {
		return new Response(JSON.stringify({
			error: 'model_not_available',
			message: `${body.modelType} is not available on the ${tier} plan.`,
		}), {
			status: 403,
			headers: { 'Content-Type': 'application/json', 'Access-Control-Allow-Origin': '*' },
		});
	}

	// Check daily limit (skip for unlimited tiers where daily_limit = -1)
	const currentUsed = usageToday[body.modelType] ?? 0;
	if (modelLimit.daily_limit >= 0 && currentUsed >= modelLimit.daily_limit) {
		return new Response(JSON.stringify({
			error: 'rate_limit_exceeded',
			message: `Daily limit of ${modelLimit.daily_limit} requests reached for ${body.modelType}. Resets at midnight UTC.`,
			daily_limit: modelLimit.daily_limit,
			requests_used: currentUsed,
		}), {
			status: 429,
			headers: { 'Content-Type': 'application/json', 'Access-Control-Allow-Origin': '*' },
		});
	}

	// Increment usage for this turn
	if (modelLimit.daily_limit >= 0) {
		await incrementUsage(admin, user.id, body.modelType, currentUsed);
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
	// Otherwise, fall back to per-call rate limiting for backwards compatibility.
	let usageToday: Record<string, number> = {};

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
		// Legacy path: per-call rate limiting (no session token)
		const [tier, fetchedUsage] = await Promise.all([
			getUserTier(admin, user.id),
			getAllUsageToday(admin, user.id),
		]);
		usageToday = fetchedUsage;

		const modelLimit = await getModelLimitForTier(admin, tier, body.modelType);

		if (!modelLimit || !modelLimit.is_available) {
			return new Response(JSON.stringify({
				error: 'model_not_available',
				message: `${body.modelType} is not available on the ${tier} plan.`,
			}), {
				status: 403,
				headers: { 'Content-Type': 'application/json', 'Access-Control-Allow-Origin': '*' },
			});
		}

		if (modelLimit.daily_limit >= 0) {
			const used = usageToday[body.modelType] ?? 0;
			if (used >= modelLimit.daily_limit) {
				return new Response(JSON.stringify({
					error: 'rate_limit_exceeded',
					message: `Daily limit of ${modelLimit.daily_limit} requests reached for ${body.modelType}. Resets at midnight UTC.`,
					daily_limit: modelLimit.daily_limit,
					requests_used: used,
				}), {
					status: 429,
					headers: { 'Content-Type': 'application/json', 'Access-Control-Allow-Origin': '*' },
				});
			}
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

		// Only increment usage for legacy (non-session) calls
		if (!body.sessionToken) {
			const currentUsed = usageToday[body.modelType] ?? 0;
			ctx.waitUntil(incrementUsage(admin, user.id, body.modelType, currentUsed).catch(e =>
				console.error('Failed to increment usage:', e)
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

		// Only increment usage for legacy (non-session) calls
		if (!body.sessionToken) {
			const currentUsed = usageToday[body.modelType] ?? 0;
			ctx.waitUntil(incrementUsage(admin, user.id, body.modelType, currentUsed).catch(e =>
				console.error('Failed to increment usage:', e)
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
