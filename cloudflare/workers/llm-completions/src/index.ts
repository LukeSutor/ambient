import { Environment, GenerateContentConfig, GoogleGenAI, ThinkingLevel } from "@google/genai";
import { createClient } from '@supabase/supabase-js'

type requestData = {
	modelType: string;
	content: Array<{ role: string; parts: object[] }>;
	stream: boolean;
	token: string;
	systemPrompt?: string;
	jsonSchema?: object;
}

const extractModelName = (modelType: string): string | null => {
	if (modelType === "fast")
		return "gemini-3-flash-preview";
	if (modelType === "pro")
		return "gemini-3-pro-preview";
	if (modelType === "computer-use")
		return "gemini-2.5-computer-use-preview-10-2025";
	return null;
};

export default {
	async fetch(request, env, ctx): Promise<Response> {
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

		// Ensure use is authenticated
		const supabase = createClient(env["SUPABASE_URL"], env["SUPABASE_ANON_KEY"]);
		const { data: { user } } = await supabase.auth.getUser(body.token);

		if (!user) {
			return new Response('Unauthorized: Invalid token', { status: 401 });
		}

		// Map model type to model name
		const modelName = extractModelName(body.modelType);
		if (!modelName) {
			return new Response('Bad Request: Invalid model type', { status: 400 });
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
		if (body.modelType === "computer-use") {
			chatConfig.tools = [{
				computerUse: {
							environment: Environment.ENVIRONMENT_BROWSER
					}
			}]
			chatConfig.temperature = 1;
			chatConfig.topP = 0.95;
			chatConfig.topK = 40;
			chatConfig.maxOutputTokens = 8192;
		} else {
			chatConfig.systemInstruction = body.systemPrompt || "You are a helpful assistant.";
			chatConfig.thinkingConfig = {
				thinkingLevel: ThinkingLevel.MINIMAL
			};
		}

		const ai = new GoogleGenAI({ apiKey: env["GEMINI_API_KEY"] });
		if (body.stream) {
			const result = await ai.models.generateContentStream({
				model: modelName,
				contents: body.content,
				config: chatConfig
			});

			const { readable, writable } = new TransformStream();
			const writer = writable.getWriter();
			const encoder = new TextEncoder();

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
				},
			});
		} else {
			const response = await ai.models.generateContent({
				model: modelName,
				contents: body.content,
				config: chatConfig
			});
			return new Response(JSON.stringify(response.text), {
				headers: { 'Content-Type': 'application/json' },
			});
		}
	},
} satisfies ExportedHandler<Env>;
