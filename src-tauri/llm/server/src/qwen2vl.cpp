#include "arg.h"
#include "base64.hpp"
#include "log.h"
#include "common.h"
#include "sampling.h"
#include "clip.h"
#include "llava.h"
#include "llama.h"
#include "ggml.h"

#ifdef GGML_USE_CUDA
#include "ggml-cuda.h"
#endif
#ifdef NDEBUG
#include "ggml-alloc.h"
#include "ggml-backend.h"
#endif

#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <vector>
#include <algorithm>
#include <iostream>
#include <fstream>
#include <json.hpp>
#include <thread>
#include <string>
#include <atomic>
#include "json-schema-to-grammar.h"

// These two are initialized in the main method and need to be used globally
common_params params;
llama_model *model;

const std::string SHOWUI_SYSTEM_PROMPT = R"(You are an assistant trained to navigate the desktop screen. 
    Given a task instruction, a screen observation, and an action history sequence, 
    output the next action and wait for the next observation. 
    Format the action as a dictionary with the following keys:
    {'action': 'ACTION_TYPE', 'value': 'element', 'position': [x,y]}
    
    If value or position is not applicable, set it as None.
    Position might be [[x1,y1], [x2,y2]] if the action requires a start and end position.
    Position represents the relative coordinates on the screenshot and should be scaled to a range of 0-1.

    Here is the action space:
    1. CLICK: Click on an element, value is not applicable and the position [x,y] is required. 
    2. INPUT: Type a string into an element, value is a string to type and the position [x,y] is required. 
    3. HOVER: Hover on an element, value is not applicable and the position [x,y] is required.
    4. ENTER: Enter operation, value and position are not applicable.
    5. SCROLL: Scroll the screen, value is the direction to scroll and the position is not applicable.
    6. ESC: ESCAPE operation, value and position are not applicable.
    7. PRESS: Long click on an element, value is not applicable and the position [x,y] is required.
    Here is the action you must perform:
)";

const std::string SHOWUI_JSON_SCHEMA = R"({
  "$schema": "http://json-schema.org/draft-07/schema#",
  "oneOf": [
    {
      "type": "object",
      "required": ["action", "position"],
      "properties": {
        "action": { "type": "string", "const": "CLICK" },
        "value": { "type": "null" },
        "position": {
          "type": "array",
          "items": { "type": "number" },
          "minItems": 2,
          "maxItems": 2
        }
      },
      "additionalProperties": false
    },
    {
      "type": "object",
      "required": ["action", "value", "position"],
      "properties": {
        "action": { "type": "string", "const": "INPUT" },
        "value": { "type": "string" },
        "position": {
          "type": "array",
          "items": { "type": "number" },
          "minItems": 2,
          "maxItems": 2
        }
      },
      "additionalProperties": false
    },
    {
      "type": "object",
      "required": ["action", "position"],
      "properties": {
        "action": { "type": "string", "const": "HOVER" },
        "value": { "type": "null" },
        "position": {
          "type": "array",
          "items": { "type": "number" },
          "minItems": 2,
          "maxItems": 2
        }
      },
      "additionalProperties": false
    },
    {
      "type": "object",
      "required": ["action"],
      "properties": {
        "action": { "type": "string", "const": "ENTER" },
        "value": { "type": "null" },
        "position": { "type": "null" }
      },
      "additionalProperties": false
    },
    {
      "type": "object",
      "required": ["action", "value"],
      "properties": {
        "action": { "type": "string", "const": "SCROLL" },
        "value": { "type": "string" },
        "position": { "type": "null" }
      },
      "additionalProperties": false
    },
    {
      "type": "object",
      "required": ["action"],
      "properties": {
        "action": { "type": "string", "const": "ESC" },
        "value": { "type": "null" },
        "position": { "type": "null" }
      },
      "additionalProperties": false
    },
    {
      "type": "object",
      "required": ["action", "position"],
      "properties": {
        "action": { "type": "string", "const": "PRESS" },
        "value": { "type": "null" },
        "position": {
          "type": "array",
          "items": { "type": "number" },
          "minItems": 2,
          "maxItems": 2
        }
      },
      "additionalProperties": false
    }
  ]
})";

const std::string CONTROL_SYSTEM_PROMPT = R"(You are an assistant trained to navigate the desktop screen. 
Given a task instruction, a screen observation, and an action history sequence, 
output the next action and wait for the next observation.
Note that x, y positions represent the relative coordinates on the screenshot and should be scaled to a range of 0-1.
Here are the tasks you can choose from:

1. HOVER: Hover the mouse over the specified x and y coordinates. 
   - Example:
     {
        "action": "HOVER",
        "x": 0.1,
        "y": 0.27
     }

2. CLICK: Click a specified mouse button at the specified x and y coordinates. Can choose between the LEFT, RIGHT, and MIDDLE mouse buttons.
   - Example:
     {
        "action": "CLICK",
        "mouse_button": "LEFT",
        "x": 0.642,
        "y": 0.05
     }

3. TYPE: Type a given string of text in an input field at the specified x and y coordinates. This command will simulate a click, selecting the input field before it types.
   - Example:
     {
        "action": "TYPE",
        "text": "Hello, World!",
        "x": 0.4,
        "y": 0.37
     }

Generate JSON outputs based on these instructions using the correct properties for each action.)";

const std::string CONTROL_JSON_SCHEMA = R"({
  "$schema": "http://json-schema.org/draft-07/schema#",
  "oneOf": [
    {
      "type": "object",
      "required": ["action", "x", "y"],
      "properties": {
        "action": { "type": "string", "const": "HOVER" },
        "x": { "type": "number", "minimum": 0, "maximum": 1 },
        "y": { "type": "number", "minimum": 0, "maximum": 1 }
      },
      "additionalProperties": false
    },
    {
      "type": "object",
      "required": ["action", "mouse_button", "x", "y"],
      "properties": {
        "action": { "type": "string", "const": "CLICK" },
        "mouse_button": {
          "type": "string",
          "enum": ["LEFT", "RIGHT", "MIDDLE"]
        },
        "x": { "type": "number", "minimum": 0, "maximum": 1 },
        "y": { "type": "number", "minimum": 0, "maximum": 1 }
      },
      "additionalProperties": false
    },
    {
      "type": "object",
      "required": ["action", "text", "x", "y"],
      "properties": {
        "action": { "type": "string", "const": "TYPE" },
        "text": { "type": "string" },
        "x": { "type": "number", "minimum": 0, "maximum": 1 },
        "y": { "type": "number", "minimum": 0, "maximum": 1 }
      },
      "additionalProperties": false
    }
  ]
})";

static bool qwen2vl_eval_image_embed(llama_context *ctx_llama, const struct llava_image_embed *image_embed,
                                     int n_batch, int *n_past, int *st_pos_id, struct clip_image_size *image_size)
{
    int n_embd = llama_n_embd(llama_get_model(ctx_llama));
    const int patch_size = 14 * 2;
    const int ph = image_size->height / patch_size + (image_size->height % patch_size > 0);
    const int pw = image_size->width / patch_size + (image_size->width % patch_size > 0);
    auto img_tokens = image_embed->n_image_pos;
    // llama_pos mrope_pos[img_tokens * 4];
    std::vector<llama_pos> mrope_pos;
    mrope_pos.resize(img_tokens * 4);

    for (int y = 0; y < ph; y++)
    {
        for (int x = 0; x < pw; x++)
        {
            int i = y * pw + x;
            mrope_pos[i] = *st_pos_id;
            mrope_pos[i + img_tokens] = *st_pos_id + y;
            mrope_pos[i + img_tokens * 2] = *st_pos_id + x;
            mrope_pos[i + img_tokens * 3] = 0;
        }
    }
    *st_pos_id += std::max(pw, ph);

    int processed = 0;
    std::vector<llama_pos> batch_mrope_pos;
    batch_mrope_pos.resize(img_tokens * 4);

    for (int i = 0; i < img_tokens; i += n_batch)
    {
        int n_eval = img_tokens - i;
        if (n_eval > n_batch)
        {
            n_eval = n_batch;
        }

        // llama_pos batch_mrope_pos[n_eval * 4];
        std::fill(batch_mrope_pos.begin(), batch_mrope_pos.end(), 0);
        memcpy(batch_mrope_pos.data(), &mrope_pos[processed], n_eval * sizeof(llama_pos));
        memcpy(&batch_mrope_pos[n_eval * 1], &mrope_pos[img_tokens * 1 + processed], n_eval * sizeof(llama_pos));
        memcpy(&batch_mrope_pos[n_eval * 2], &mrope_pos[img_tokens * 2 + processed], n_eval * sizeof(llama_pos));
        memcpy(&batch_mrope_pos[n_eval * 3], &mrope_pos[img_tokens * 3 + processed], n_eval * sizeof(llama_pos));

        llama_batch batch = {
            int32_t(n_eval),                   // n_tokens
            nullptr,                           // token
            (image_embed->embed + i * n_embd), // embed
            batch_mrope_pos.data(),            // pos
            nullptr,                           // n_seq_id
            nullptr,                           // seq_id
            nullptr,                           // logits
        };

        if (llama_decode(ctx_llama, batch))
        {
            // LOG_ERR("%s : failed to eval\n", __func__);
            return false;
        }
        *n_past += n_eval;
        processed += n_eval;
    }
    return true;
}

static bool eval_tokens(struct llama_context *ctx_llama, std::vector<llama_token> tokens, int n_batch, int *n_past, int *st_pos_id)
{
    int N = (int)tokens.size();
    std::vector<llama_pos> pos;
    for (int i = 0; i < N; i += n_batch)
    {
        int n_eval = (int)tokens.size() - i;
        if (n_eval > n_batch)
        {
            n_eval = n_batch;
        }
        auto batch = llama_batch_get_one(&tokens[i], n_eval);
        // TODO: add mrope pos ids somewhere else
        pos.resize(batch.n_tokens * 4);
        std::fill(pos.begin(), pos.end(), 0);
        for (int j = 0; j < batch.n_tokens * 3; j++)
        {
            pos[j] = *st_pos_id + (j % batch.n_tokens);
        }
        batch.pos = pos.data();

        if (llama_decode(ctx_llama, batch))
        {
            // LOG_ERR("%s : failed to eval. token %d/%d (batch size %d, n_past %d)\n", __func__, i, N, n_batch, *n_past);
            return false;
        }
        *n_past += n_eval;
        *st_pos_id += n_eval;
    }
    return true;
}

static bool eval_id(struct llama_context *ctx_llama, int id, int *n_past, int *st_pos_id)
{
    std::vector<llama_token> tokens;
    tokens.push_back(id);
    return eval_tokens(ctx_llama, tokens, 1, n_past, st_pos_id);
}

static bool eval_string(struct llama_context *ctx_llama, const char *str, int n_batch, int *n_past, int *st_pos_id, bool add_bos)
{
    std::string str2 = str;
    std::vector<llama_token> embd_inp = common_tokenize(ctx_llama, str2, add_bos, true);
    eval_tokens(ctx_llama, embd_inp, n_batch, n_past, st_pos_id);
    return true;
}

static const char *sample(struct common_sampler *smpl,
                          struct llama_context *ctx_llama,
                          int *n_past, int *st_pos_id)
{
    const llama_token id = common_sampler_sample(smpl, ctx_llama, -1);
    common_sampler_accept(smpl, id, true);
    static std::string ret;
    if (llama_token_is_eog(llama_get_model(ctx_llama), id))
    {
        ret = "</s>";
    }
    else
    {
        ret = common_token_to_piece(ctx_llama, id);
    }
    eval_id(ctx_llama, id, n_past, st_pos_id);
    return ret.c_str();
}

static const char *IMG_BASE64_TAG_BEGIN = "<img src=\"data:image/jpeg;base64,";
static const char *IMG_BASE64_TAG_END = "\">";

static void find_image_tag_in_prompt(const std::string &prompt, size_t &begin_out, size_t &end_out)
{
    begin_out = prompt.find(IMG_BASE64_TAG_BEGIN);
    end_out = prompt.find(IMG_BASE64_TAG_END, (begin_out == std::string::npos) ? 0UL : begin_out);
}

static bool prompt_contains_image(const std::string &prompt)
{
    size_t begin, end;
    find_image_tag_in_prompt(prompt, begin, end);
    return (begin != std::string::npos);
}

// replaces the base64 image tag in the prompt with `replacement`
static llava_image_embed *llava_image_embed_make_with_prompt_base64(struct clip_ctx *ctx_clip, int n_threads, const std::string &prompt)
{
    size_t img_base64_str_start, img_base64_str_end;
    find_image_tag_in_prompt(prompt, img_base64_str_start, img_base64_str_end);
    if (img_base64_str_start == std::string::npos || img_base64_str_end == std::string::npos)
    {
        // LOG_ERR("%s: invalid base64 image tag. must be %s<base64 byte string>%s\n", __func__, IMG_BASE64_TAG_BEGIN, IMG_BASE64_TAG_END);
        return NULL;
    }

    auto base64_bytes_start = img_base64_str_start + strlen(IMG_BASE64_TAG_BEGIN);
    auto base64_bytes_count = img_base64_str_end - base64_bytes_start;
    auto base64_str = prompt.substr(base64_bytes_start, base64_bytes_count);

    auto required_bytes = base64::required_encode_size(base64_str.size());
    auto img_bytes = std::vector<unsigned char>(required_bytes);
    base64::decode(base64_str.begin(), base64_str.end(), img_bytes.begin());

    auto embed = llava_image_embed_make_with_bytes(ctx_clip, n_threads, img_bytes.data(), img_bytes.size());
    if (!embed)
    {
        // LOG_ERR("%s: could not load image from base64 string.\n", __func__);
        return NULL;
    }

    return embed;
}

static std::string remove_image_from_prompt(const std::string &prompt, const char *replacement = "")
{
    size_t begin, end;
    find_image_tag_in_prompt(prompt, begin, end);
    if (begin == std::string::npos || end == std::string::npos)
    {
        return prompt;
    }
    auto pre = prompt.substr(0, begin);
    auto post = prompt.substr(end + strlen(IMG_BASE64_TAG_END));
    return pre + replacement + post;
}

struct llava_context
{
    struct clip_ctx *ctx_clip = NULL;
    struct llama_context *ctx_llama = NULL;
    struct llama_model *model = NULL;
};

static struct llava_image_embed *load_image(llava_context *ctx_llava, common_params *params, const std::string &fname)
{

    // load and preprocess the image
    llava_image_embed *embed = NULL;
    auto prompt = params->prompt;
    if (prompt_contains_image(prompt))
    {
        if (!params->image.empty())
        {
            // LOG_INF("using base64 encoded image instead of command line image path\n");
        }
        embed = llava_image_embed_make_with_prompt_base64(ctx_llava->ctx_clip, params->cpuparams.n_threads, prompt);
        if (!embed)
        {
            // LOG_ERR("%s: can't load image from prompt\n", __func__);
            return NULL;
        }
        params->prompt = remove_image_from_prompt(prompt);
    }
    else
    {
        embed = llava_image_embed_make_with_filename(ctx_llava->ctx_clip, params->cpuparams.n_threads, fname.c_str());
        if (!embed)
        {
            fprintf(stderr, "%s: is %s really an image file?\n", __func__, fname.c_str());
            return NULL;
        }
    }

    return embed;
}

static std::string process_prompt(struct llava_context *ctx_llava, struct llava_image_embed *image_embed, common_params *params, const std::string &prompt)
{
    int n_past = 0;
    int cur_pos_id = 0;

    const int max_tgt_len = params->n_predict < 0 ? 256 : params->n_predict;

    std::string system_prompt, user_prompt;
    size_t image_pos = prompt.find("<|vision_start|>");
    if (image_pos != std::string::npos)
    {
        // new templating mode: Provide the full prompt including system message and use <image> as a placeholder for the image
        system_prompt = prompt.substr(0, image_pos);
        user_prompt = prompt.substr(image_pos + std::string("<|vision_pad|>").length());
        // LOG_INF("system_prompt: %s\n", system_prompt.c_str());
        if (params->verbose_prompt)
        {
            auto tmp = common_tokenize(ctx_llava->ctx_llama, system_prompt, true, true);
            for (int i = 0; i < (int)tmp.size(); i++)
            {
                // LOG_INF("%6d -> '%s'\n", tmp[i], common_token_to_piece(ctx_llava->ctx_llama, tmp[i]).c_str());
            }
        }
        // LOG_INF("user_prompt: %s\n", user_prompt.c_str());
        if (params->verbose_prompt)
        {
            auto tmp = common_tokenize(ctx_llava->ctx_llama, user_prompt, true, true);
            for (int i = 0; i < (int)tmp.size(); i++)
            {
                // LOG_INF("%6d -> '%s'\n", tmp[i], common_token_to_piece(ctx_llava->ctx_llama, tmp[i]).c_str());
            }
        }
    }
    else
    {
        // llava-1.5 native mode

        // Only include the vision tokens if an image is passed
        if (image_embed != nullptr)
        {
            system_prompt = "<|im_start|>system\n" + CONTROL_SYSTEM_PROMPT + "<|im_end|>\n<|im_start|>user\n<|vision_start|>";
            user_prompt = "<|vision_end|>" + prompt + "<|im_end|>\n<|im_start|>assistant\n";
        }
        else
        {
            system_prompt = "<|im_start|>system\n" + CONTROL_SYSTEM_PROMPT + "<|im_end|>\n<|im_start|>user\n";
            user_prompt = prompt + "<|im_end|>\n<|im_start|>assistant\n";
        }
        if (params->verbose_prompt)
        {
            auto tmp = common_tokenize(ctx_llava->ctx_llama, user_prompt, true, true);
            for (int i = 0; i < (int)tmp.size(); i++)
            {
                // LOG_INF("%6d -> '%s'\n", tmp[i], common_token_to_piece(ctx_llava->ctx_llama, tmp[i]).c_str());
            }
        }
    }

    eval_string(ctx_llava->ctx_llama, system_prompt.c_str(), params->n_batch, &n_past, &cur_pos_id, true);
    if (image_embed != nullptr)
    {
        auto image_size = clip_get_load_image_size(ctx_llava->ctx_clip);
        qwen2vl_eval_image_embed(ctx_llava->ctx_llama, image_embed, params->n_batch, &n_past, &cur_pos_id, image_size);
    }
    eval_string(ctx_llava->ctx_llama, user_prompt.c_str(), params->n_batch, &n_past, &cur_pos_id, false);

    // generate the response

    // LOG("\n");

    struct common_sampler *smpl = common_sampler_init(ctx_llava->model, params->sampling);
    if (!smpl)
    {
        // LOG_ERR("%s: failed to initialize sampling subsystem\n", __func__);
        exit(1);
    }

    std::string response = "";
    for (int i = 0; i < max_tgt_len; i++)
    {
        const char *tmp = sample(smpl, ctx_llava->ctx_llama, &n_past, &cur_pos_id);
        response += tmp;
        if (strcmp(tmp, "</s>") == 0)
            break;
        if (strstr(tmp, "###"))
            break; // Yi-VL behavior
        // LOG("%s", tmp);
        if (strstr(response.c_str(), "<|im_end|>"))
            break; // Yi-34B llava-1.6 - for some reason those decode not as the correct token (tokenizer works)
        if (strstr(response.c_str(), "<|im_start|>"))
            break; // Yi-34B llava-1.6
        if (strstr(response.c_str(), "USER:"))
            break; // mistral llava-1.6

        fflush(stdout);
    }

    common_sampler_free(smpl);
    // LOG("\n");
    // LOG("\nFinal response: %s\n", response.c_str()); // Debug log final response
    if (response.length() >= 4 && response.substr(response.length() - 4) == "</s>")
    {
        response = response.substr(0, response.length() - 4);
    }
    return response;
}

static struct llama_model *llava_init(common_params *params)
{
    llama_backend_init();
    llama_numa_init(params->numa);

    llama_model_params model_params = common_model_params_to_llama(*params);

    llama_model *model = llama_load_model_from_file(params->model.c_str(), model_params);
    if (model == NULL)
    {
        // LOG_ERR("%s: unable to load model\n", __func__);
        return NULL;
    }
    return model;
}

static struct llava_context *llava_init_context(common_params *params, llama_model *model)
{
    const char *clip_path = params->mmproj.c_str();

    auto prompt = params->prompt;
    if (prompt.empty())
    {
        prompt = "describe the image in detail.";
    }

    auto ctx_clip = clip_model_load(clip_path, /*verbosity=*/1);

    llama_context_params ctx_params = common_context_params_to_llama(*params);
    ctx_params.n_ctx = params->n_ctx < 2048 ? 2048 : params->n_ctx; // we need a longer context size to process image embeddings

    llama_context *ctx_llama = llama_new_context_with_model(model, ctx_params);

    if (ctx_llama == NULL)
    {
        // LOG_ERR("%s: failed to create the llama_context\n", __func__);
        return NULL;
    }

    auto *ctx_llava = (struct llava_context *)malloc(sizeof(llava_context));

    ctx_llava->ctx_llama = ctx_llama;
    ctx_llava->ctx_clip = ctx_clip;
    ctx_llava->model = model;
    return ctx_llava;
}

static void llava_free(struct llava_context *ctx_llava)
{
    if (ctx_llava->ctx_clip)
    {
        clip_free(ctx_llava->ctx_clip);
        ctx_llava->ctx_clip = NULL;
    }

    llama_free(ctx_llava->ctx_llama);
    llama_free_model(ctx_llava->model);
    llama_backend_free();
}

void log_file(const std::string &input) {
    std::ofstream log_file("C:\\Users\\Luke\\Downloads\\log.txt", std::ios_base::app);
    if (log_file.is_open())
    {
        log_file << input << std::endl;
    }
}

std::string load_model(const std::string &data) {
    try {
        // Extract the model paths
        auto json = nlohmann::json::parse(data);

        if (!json.contains("text_model") || !json.contains("vision_model")) {
            nlohmann::json response = {
                {"success", false},
                {"reason", "Missing required 'text_model' or 'vision_model' field"}};
            return response.dump();
        }

        std::string text_model = json["text_model"];
        std::string vision_model = json["vision_model"];

        // Load the models
        params.model = text_model;
        params.mmproj = vision_model;
        if (model != nullptr) {
            llama_free_model(model);
        }
        model = llava_init(&params);

        nlohmann::json response = {
            {"success", true},
            {"reason", "Models loaded successfully"}};
        return response.dump();
    } catch (const nlohmann::json::parse_error &e) {
        nlohmann::json response = {
            {"success", false},
            {"reason", "Invalid JSON payload"}};
        return response.dump();
    }
}

std::string infer(const std::string &data) {
    // Make sure the model is loaded
    if (model == nullptr) {
        nlohmann::json response = {
            {"success", false},
            {"reason", "Model not loaded"}};
        return response.dump();
    }
    try
    {
        // Parse JSON from request body
        auto json = nlohmann::json::parse(data);

        // Check for required prompt field
        if (!json.contains("prompt"))
        {
            nlohmann::json response = {
                {"success", false},
                {"reason", "Missing required 'prompt' field"}};
            return response.dump();
        }

        // Extract fields
        std::string prompt = json["prompt"];
        std::string image = json.value("image", ""); // Optional field

        // Generate with Qwen
        params.prompt = prompt;
        std::string result = "";
        if (image.empty())
        {

            // Generate without image input
            llava_context *ctx_llava = llava_init_context(&params, model);

            // process the prompt
            result = process_prompt(ctx_llava, nullptr, &params, prompt);

            ctx_llava->model = NULL;
            llava_free(ctx_llava);
        }
        else
        {
            // Generate with image input
            llava_context *ctx_llava = llava_init_context(&params, model);
            llava_image_embed *image_embed = load_image(ctx_llava, &params, image);
            if (!image_embed)
            {
                nlohmann::json response = {
                    {"success", false},
                    {"reason", "Failed to load image %s", image.c_str()}};
                return response.dump();
            }

            // process the prompt
            result = process_prompt(ctx_llava, image_embed, &params, prompt);

            llava_image_embed_free(image_embed);
            ctx_llava->model = NULL;
            llava_free(ctx_llava);
        }
        params.prompt = "";
        
        // Add success to the json and return
        auto result_json = nlohmann::json::parse(result);
        result_json["success"] = true;
        return result_json.dump();
    }
    catch (const nlohmann::json::parse_error &e)
    {
        nlohmann::json response = {
            {"success", false},
            {"reason", "Invalid JSON payload"}};
        return response.dump();
    }
}

void processRequest(std::atomic<bool> &running)
{
    std::string input;
    while (running)
    {
        std::getline(std::cin, input);
        std::cin.clear(); // Clear the input buffer
        if (!input.empty())
        {
            if (input == "SHUTDOWN")
            {
                nlohmann::json response = {
                    {"success", true},
                    {"reason", "Shutting down"}};
                std::cout << "RESPONSE " << response.dump() << std::endl;
                running = false;
                break;
            }
            else if (input.rfind("INFER", 0) == 0)
            {
                std::string response = infer(input.substr(6)); // Call infer with the rest of the input
                std::cout << "RESPONSE " << response << std::endl;
            }
            else if (input.rfind("LOAD", 0) == 0)
            {
                std::string response = load_model(input.substr(5)); // Call load_model with the rest of the input
                std::cout << "RESPONSE " << response << std::endl;
            }
            else
            {
                nlohmann::json response = {
                    {"success", false},
                    {"reason", "Error unknown function: " + input}};
                std::cout << "RESPONSE " << response.dump() << std::endl;
            }
            input = "";
        }
    }
}

int main()
{
    // Qwen Model initialization
    // TODO: make the n_threads dependent on whether user is running the system in foreground or background
    params.cpuparams.n_threads = 4;
    params.sampling.grammar = json_schema_to_grammar(nlohmann::json::parse(CONTROL_JSON_SCHEMA));

    // Listen to stdin in another thread and respond to requests
    std::atomic<bool> running(true);
    std::thread listener(processRequest, std::ref(running));
    listener.join();

    llama_free_model(model);
    return 0;
}
