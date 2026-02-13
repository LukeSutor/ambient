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
 * Uses a select-then-upsert pattern because the Supabase JS client
 * doesn't support SQL-level `SET x = x + 1` in upserts.
 *
 * TODO: For atomic increment, create a Supabase Postgres function:
 *
 *   CREATE OR REPLACE FUNCTION increment_model_usage(p_user_id uuid, p_model_type text)
 *   RETURNS void AS $$
 *   BEGIN
 *     INSERT INTO model_usage (user_id, model_type, usage_date, requests_used)
 *     VALUES (p_user_id, p_model_type, CURRENT_DATE, 1)
 *     ON CONFLICT (user_id, model_type, usage_date)
 *     DO UPDATE SET requests_used = model_usage.requests_used + 1;
 *   END;
 *   $$ LANGUAGE plpgsql;
 *
 * Then replace this function body with:
 *   await admin.rpc('increment_model_usage', { p_user_id: userId, p_model_type: modelType });
 *
 * This requires a UNIQUE constraint on (user_id, model_type, usage_date).
 */
async function incrementUsage(admin: SupabaseClient, userId: string, modelType: string): Promise<void> {
	const today = getTodayUTC();

	// Try to increment existing row first
	const { data: existing } = await admin
		.from('model_usage')
		.select('id, requests_used')
		.eq('user_id', userId)
		.eq('model_type', modelType)
		.eq('usage_date', today)
		.maybeSingle();

	if (existing) {
		const { error } = await admin
			.from('model_usage')
			.update({ requests_used: existing.requests_used + 1 })
			.eq('id', existing.id);
		if (error) {
			console.error('Failed to increment usage:', error);
			throw new Error('Failed to increment usage');
		}
	} else {
		const { error } = await admin
			.from('model_usage')
			.insert({
				user_id: userId,
				model_type: modelType,
				usage_date: today,
				requests_used: 1,
			});
		if (error) {
			console.error('Failed to insert usage:', error);
			throw new Error('Failed to insert usage');
		}
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

	// --- Tier-aware rate limit check ---
	// Fetch tier and today's usage for this model in parallel.
	const admin = createAdminClient(env);
	const [tier, usageToday] = await Promise.all([
		getUserTier(admin, user.id),
		getAllUsageToday(admin, user.id),
	]);

	// Only fetch the single model's limit (not all models)
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

		// Increment usage immediately — stream has started successfully
		ctx.waitUntil(incrementUsage(admin, user.id, body.modelType).catch(e =>
			console.error('Failed to increment usage:', e)
		));

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

		// Increment usage — generation succeeded
		ctx.waitUntil(incrementUsage(admin, user.id, body.modelType).catch(e =>
			console.error('Failed to increment usage:', e)
		));

		return new Response(JSON.stringify(response), {
			headers: { 
				'Content-Type': 'application/json',
				'Access-Control-Allow-Origin': '*',
			},
		});
	}
}
