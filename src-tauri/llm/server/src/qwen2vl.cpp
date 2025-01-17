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

//TODO: Update to replace with user OS
const std::string PLANNER_SYSTEM_PROMPT = R"(You are using a Windows device.
You are able to use a mouse and keyboard to interact with the computer based on the given task and screenshot.
You can only interact with the desktop GUI (no terminal or application menu access).

You may be given some history plan and actions, this is the response from the previous loop.
You should carefully consider your plan base on the task, screenshot, and history actions.

Your available "Next Action" only include:
- ENTER: Press an enter key.
- ESCAPE: Press an ESCAPE key.
- INPUT: Input a string of text.
- CLICK: Describe the ui element to be clicked.
- HOVER: Describe the ui element to be hovered.
- SCROLL: Scroll the screen, you must specify up or down.
- PRESS: Describe the ui element to be pressed.

Output format:
```json
{{
    "Thinking": str, # describe your thoughts on how to achieve the task, choose one action from available actions at a time.
    "Next Action": "action_type, action description" | "None" # one action at a time, describe it in short and precisely. 
}}
```

One Example:
```json
{{  
    "Thinking": "I need to search and navigate to amazon.com.",
    "Next Action": "CLICK 'Search Google or type a URL'."
}}
```

IMPORTANT NOTES:
1. Carefully observe the screenshot to understand the current state and read history actions.
2. You should only give a single action at a time. for example, INPUT text, and ENTER can't be in one Next Action.
3. Attach the text to Next Action, if there is text or any description for the button. 
4. You should not include other actions, such as keyboard shortcuts.
5. When the task is completed, you should say "Next Action": "None" in the json field.)";

const std::string PLANNER_JSON_SCHEMA = R"({
  "$schema": "http://json-schema.org/draft-07/schema#",
  "type": "object",
  "properties": {
    "Thinking": {
      "type": "string",
      "description": "Describe your thoughts on how to achieve the task, choose one action from available actions at a time."
    },
    "Next Action": {
      "type": "string",
      "description": "One action at a time, describe it in short and precisely. Format: 'action_type, action description' or 'None'."
    }
  },
  "required": ["Thinking", "Next Action"],
  "additionalProperties": false
})";

const std::string EXECUTOR_SYSTEM_PROMPT = R"(You are an assistant trained to navigate the desktop screen. 
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

const std::string EXECUTOR_JSON_SCHEMA = R"({
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

// enum inference_mode {
//     PLANNER = 0,
//     EXECUTOR = 1
// };

// struct llava_context
// {
//     struct clip_ctx *ctx_clip = NULL;
//     struct llama_context *ctx_llama = NULL;
//     struct llama_model *model = NULL;
// };

// struct inference_data {
//     common_params *params;
//     llama_model *model;
//     llava_context *ctx_llava;
//     llava_image_embed *image_embed;
//     inference_mode mode;
//     std::string prompt;
//     std::string image;
// };

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

enum inference_mode {
    PLANNER = 0,
    EXECUTOR = 1
};

struct inference_data {
    common_params *params;
    llama_model *model;
    llava_context *ctx_llava;
    llava_image_embed *image_embed;
    inference_mode mode;
    std::string prompt;
    std::string image;
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

static std::string process_prompt(inference_data &inference_data)//, struct llava_image_embed *image_embed, common_params *params, const std::string &prompt, inference_mode mode)
{
    int n_past = 0;
    int cur_pos_id = 0;

    const int max_tgt_len = inference_data.params->n_predict < 0 ? 256 : inference_data.params->n_predict;

    std::string system_prompt, user_prompt;
    size_t image_pos = inference_data.prompt.find("<|vision_start|>");
    if (image_pos != std::string::npos)
    {
        // new templating mode: Provide the full prompt including system message and use <image> as a placeholder for the image
        system_prompt = inference_data.prompt.substr(0, image_pos);
        user_prompt = inference_data.prompt.substr(image_pos + std::string("<|vision_pad|>").length());
        // LOG_INF("system_prompt: %s\n", system_prompt.c_str());
        if (inference_data.params->verbose_prompt)
        {
            auto tmp = common_tokenize(inference_data.ctx_llava->ctx_llama, system_prompt, true, true);
            for (int i = 0; i < (int)tmp.size(); i++)
            {
                // LOG_INF("%6d -> '%s'\n", tmp[i], common_token_to_piece(ctx_llava->ctx_llama, tmp[i]).c_str());
            }
        }
        // LOG_INF("user_prompt: %s\n", user_prompt.c_str());
        if (inference_data.params->verbose_prompt)
        {
            auto tmp = common_tokenize(inference_data.ctx_llava->ctx_llama, user_prompt, true, true);
            for (int i = 0; i < (int)tmp.size(); i++)
            {
                // LOG_INF("%6d -> '%s'\n", tmp[i], common_token_to_piece(ctx_llava->ctx_llama, tmp[i]).c_str());
            }
        }
    }
    else
    {
        // llava-1.5 native mode

        std::string general_prompt = inference_data.mode == PLANNER ? PLANNER_SYSTEM_PROMPT : EXECUTOR_SYSTEM_PROMPT;

        // Only include the vision tokens if an image is passed
        if (inference_data.image_embed != nullptr)
        {
            system_prompt = "<|im_start|>system\n" + general_prompt + "<|im_end|>\n<|im_start|>user\n<|vision_start|>";
            user_prompt = "<|vision_end|>" + inference_data.prompt + "<|im_end|>\n<|im_start|>assistant\n";
        }
        else
        {
            system_prompt = "<|im_start|>system\n" + general_prompt + "<|im_end|>\n<|im_start|>user\n";
            user_prompt = inference_data.prompt + "<|im_end|>\n<|im_start|>assistant\n";
        }
        if (inference_data.params->verbose_prompt)
        {
            auto tmp = common_tokenize(inference_data.ctx_llava->ctx_llama, user_prompt, true, true);
            for (int i = 0; i < (int)tmp.size(); i++)
            {
                // LOG_INF("%6d -> '%s'\n", tmp[i], common_token_to_piece(ctx_llava->ctx_llama, tmp[i]).c_str());
            }
        }
    }

    // Set the JSON schema
    inference_data.params->sampling.grammar = inference_data.mode == PLANNER ? json_schema_to_grammar(nlohmann::json::parse(PLANNER_JSON_SCHEMA)) : json_schema_to_grammar(nlohmann::json::parse(EXECUTOR_JSON_SCHEMA));

    eval_string(inference_data.ctx_llava->ctx_llama, system_prompt.c_str(), inference_data.params->n_batch, &n_past, &cur_pos_id, true);
    if (inference_data.image_embed != nullptr)
    {
        auto image_size = clip_get_load_image_size(inference_data.ctx_llava->ctx_clip);
        qwen2vl_eval_image_embed(inference_data.ctx_llava->ctx_llama, inference_data.image_embed, inference_data.params->n_batch, &n_past, &cur_pos_id, image_size);
    }
    eval_string(inference_data.ctx_llava->ctx_llama, user_prompt.c_str(), inference_data.params->n_batch, &n_past, &cur_pos_id, false);

    // generate the response

    struct common_sampler *smpl = common_sampler_init(inference_data.ctx_llava->model, inference_data.params->sampling);
    if (!smpl)
    {
        return "";
    }

    std::string response = "";
    for (int i = 0; i < max_tgt_len; i++)
    {
        const char *tmp = sample(smpl, inference_data.ctx_llava->ctx_llama, &n_past, &cur_pos_id);
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

std::string load_model(const std::string &data, inference_data inference_data) {
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
        inference_data.params->model = text_model;
        inference_data.params->mmproj = vision_model;
        if (inference_data.model != nullptr) {
            llama_free_model(inference_data.model);
        }
        inference_data.model = llava_init(inference_data.params);

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

// Extracts inference parameters from the request (creates data on the heap)
bool extract_params(const std::string &data, inference_data inference_data) {
    try {
        auto json = nlohmann::json::parse(data);

        // Check for required prompt field
        if (!json.contains("prompt") || !json.contains("image"))
        {
            return false;
        }

        // Extract fields
        std::string prompt = json["prompt"];
        std::string image = json["image"];

        inference_data.prompt = prompt;
        inference_data.image = image;
        return true;
    }
    catch (const nlohmann::json::parse_error &e) {
        return false;
    }
}

bool turn_setup(inference_data inference_data) {
    // Create the context
    inference_data.ctx_llava = llava_init_context(inference_data.params, inference_data.model);

    // Embed the image
    llava_image_embed *image_embed = load_image(inference_data.ctx_llava, inference_data.params, inference_data.image);
    if (image_embed == nullptr)
    {
        return false;
    }
    return true;
}

void turn_cleanup(inference_data inference_data) {
    // Free the image embedding and llava context
    llava_image_embed_free(inference_data.image_embed);
    inference_data.ctx_llava->model = NULL;
    llava_free(inference_data.ctx_llava);
}

std::string planner_turn(const std::string &data, inference_data inference_data) {
    if (!extract_params(data, inference_data)) {
        nlohmann::json response = {
            {"success", false},
            {"reason", "Invalid JSON payload, payload must contain \"prompt\" and \"image\" fields"}};
        return response.dump();
    }

    if(!turn_setup(inference_data)) {
    nlohmann::json response = {
            {"success", false},
            {"reason", "Could not initialize turn, please try again"}};
        return response.dump();    
    }

    inference_data.mode = PLANNER;

    std::string completion = infer(inference_data);
    try {
        auto result_json = nlohmann::json::parse(completion);
        return result_json.dump();
    } catch (const nlohmann::json::parse_error &e) {
        nlohmann::json response = {
            {"success", false},
            {"reason", "Invalid model response, please try again"}};
        return response.dump();
    }
}

std::string executor_turn(const std::string &data, inference_data inference_data) {
    if (!extract_params(data, inference_data)) {
        nlohmann::json response = {
            {"success", false},
            {"reason", "Invalid JSON payload, payload must contain \"prompt\" and \"image\" fields"}};
        return response.dump();
    }

    inference_data.mode = EXECUTOR;

    std::string completion = infer(inference_data);

    // Clean up
    turn_cleanup(inference_data);

    try {
        auto result_json = nlohmann::json::parse(completion);
        return result_json.dump();
    } catch (const nlohmann::json::parse_error &e) {
        nlohmann::json response = {
            {"success", false},
            {"reason", "Invalid model response, please try again"}};
        return response.dump();
    }
}

std::string infer(inference_data inference_data) {
    // Make sure the model is loaded
    if (inference_data.model == nullptr) {
        nlohmann::json response = {
            {"success", false},
            {"reason", "Model not loaded"}};
        return response.dump();
    }

    // Generate with Qwen
    inference_data.params->prompt = inference_data.prompt;
    std::string result = "";

    result = process_prompt(inference_data);
    inference_data.params->prompt = "";
    
    // If the model outputs JSON, add success to it
    try {
        auto result_json = nlohmann::json::parse(result);
        result_json["success"] = true;
        return result_json.dump();
    } catch (const nlohmann::json::parse_error &e) {
        return result;
    }
}

void processRequest(std::atomic<bool> &running, std::atomic<inference_data> &inference_data)
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
            else if (input.rfind("PLAN", 0) == 0) {
                std::string response = planner_turn(input.substr(5), inference_data); // Call infer with the rest of the input
                std::cout << "RESPONSE " << response << std::endl;
            }
            else if (input.rfind("EXECUTE", 0) == 0) {
                std::string response = executor_turn(input.substr(8), inference_data); // Call infer with the rest of the input
                std::cout << "RESPONSE " << response << std::endl;
            }
            // else if (input.rfind("INFER", 0) == 0)
            // {
                // std::string response = infer(inference_data); // Call infer with the rest of the input
                // std::cout << "RESPONSE " << response << std::endl;
            // }
            else if (input.rfind("LOAD", 0) == 0)
            {
                std::string response = load_model(input.substr(5), inference_data); // Call load_model with the rest of the input
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
    // Inference data initialization
    common_params params;
    params.cpuparams.n_threads = 4;
    inference_data data = {
        &params, // parameters
        nullptr, // model
        nullptr, // llava context
        nullptr, // image_embed
        PLANNER, // inference_mode
        "",      // prompt
        ""       // image
    };
    // TODO: make the n_threads dependent on whether user is running the system in foreground or background

    // Listen to stdin in another thread and respond to requests
    std::atomic<bool> running(true);
    std::atomic<inference_data> inference_data(data);
    std::thread listener(processRequest, std::ref(running), std::ref(inference_data));
    listener.join();

    llama_free_model(inference_data.load().model);
    return 0;
}
