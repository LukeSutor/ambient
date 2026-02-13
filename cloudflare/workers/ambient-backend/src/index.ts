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

// Daily request limits per model type
const MODEL_LIMITS: Record<string, number> = {
	fast: 3,
	pro: 0, // Pro is behind paywall — no free uses
};

const extractModelName = (modelType: string): string | null => {
	if (modelType === "fast")
		return "gemini-3-flash-preview";
	if (modelType === "pro")
		return "gemini-3-pro-preview";
	return null;
};

/** Get today's date in YYYY-MM-DD format (UTC) */
function getTodayUTC(): string {
	return new Date().toISOString().split('T')[0];
}

/** Create a Supabase admin client (service role — bypasses RLS) */
function createAdminClient(env: Env): SupabaseClient {
	return createClient(env.SUPABASE_URL, env.SUPABASE_SERVICE_ROLE_KEY);
}

/**
 * Check how many requests a user has used today for a given model.
 * Returns the number of requests used, or 0 if no row exists yet.
 */
async function getRequestsUsedToday(admin: SupabaseClient, userId: string, modelType: string): Promise<number> {
	const today = getTodayUTC();
	const { data, error } = await admin
		.from('model_usage')
		.select('requests_used')
		.eq('user_id', userId)
		.eq('model_type', modelType)
		.eq('usage_date', today)
		.maybeSingle();

	if (error) {
		console.error('Failed to check usage:', error);
		throw new Error('Failed to check usage');
	}

	return data?.requests_used ?? 0;
}

/**
 * Increment the request count for a user/model/today.
 * Uses upsert with ON CONFLICT to atomically create or increment.
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
 * Handle remaining uses query
 */
async function handleGetRemainingUses(request: Request, env: Env): Promise<Response> {
	const user = await verifySupabaseUser(request, env);
	if (!user) {
		return new Response('Unauthorized: Invalid token', { status: 401 });
	}

	const admin = createAdminClient(env);
	const remaining: Record<string, { daily_limit: number; requests_used: number; remaining: number }> = {};

	for (const [modelType, limit] of Object.entries(MODEL_LIMITS)) {
		const used = await getRequestsUsedToday(admin, user.id, modelType);
		remaining[modelType] = {
			daily_limit: limit,
			requests_used: used,
			remaining: Math.max(0, limit - used),
		};
	}

	return new Response(JSON.stringify(remaining), {
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

	// --- Rate limit check ---
	const dailyLimit = MODEL_LIMITS[body.modelType];
	if (dailyLimit !== undefined) {
		try {
			const admin = createAdminClient(env);
			const used = await getRequestsUsedToday(admin, user.id, body.modelType);
			if (used >= dailyLimit) {
				return new Response(JSON.stringify({
					error: 'rate_limit_exceeded',
					message: `Daily limit of ${dailyLimit} requests reached for ${body.modelType}. Resets at midnight UTC.`,
					daily_limit: dailyLimit,
					requests_used: used,
				}), {
					status: 429,
					headers: {
						'Content-Type': 'application/json',
						'Access-Control-Allow-Origin': '*',
					},
				});
			}
		} catch (e) {
			console.error('Rate limit check failed, allowing request:', e);
			// Fail open — if rate limit check fails, allow the request
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

	// Thinking level minimal is not supported on pro, only on fast
	chatConfig.thinkingConfig = {
		thinkingLevel: body.modelType === "fast" ? ThinkingLevel.MINIMAL : ThinkingLevel.LOW
	};		

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
		const admin = createAdminClient(env);
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
		const admin = createAdminClient(env);
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
