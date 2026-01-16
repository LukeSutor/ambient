import { GoogleGenAI } from "@google/genai";
import { createClient } from '@supabase/supabase-js'

type requestData = {
	modelType: string;
	messages: Array<{ role: string; content: string }>;
	jsonSchema?: object;
	stream: boolean;
	token: string;
}

const extractModelName = (modelType: string): string | null => {
	if (modelType === "fast")
		return "gemini-3-flash";
	if (modelType === "thinking")
		return "gemini-3-pro";
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
		console.log('Request data received:', body);

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

		// Remove system message if present
		let systemInstruction = "";
		if (body.messages.length > 0 && body.messages[0].role === "system") {
			systemInstruction = body.messages[0].content;
			body.messages.shift();
		}

		const history = body.messages.map((msg) => { return { role: msg.role, parts: [{text: msg.content}] }; });

		// Handle model response
		const ai = new GoogleGenAI({apiKey: env["GEMINI_API_KEY"]});
		const chat = ai.chats.create({model: modelName, history: history, config: {systemInstruction: systemInstruction}});

		return new Response('Hello Worlsd!');
	},
} satisfies ExportedHandler<Env>;
