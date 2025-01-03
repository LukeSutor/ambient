#include "qwen_functions.h"
// #include "llava.h"
// #include "clip.h"
// #include "arg.h"
// #include "base64.hpp"
// #include "log.h"
// #include "common.h"
// #include "sampling.h"
// #include "llama.h"
// #include "ggml.h"

// #ifdef GGML_USE_CUDA
// #include "ggml-cuda.h"
// #endif
// #ifdef NDEBUG
// #include "ggml-alloc.h"
// #include "ggml-backend.h"
// #endif

// #include <cstdio>
// #include <cstdlib>
// #include <cstring>
// #include <vector>
// #include <algorithm>
// #include <iostream>
// #include <fstream>


int main(int argc, char ** argv) {
    ggml_time_init();

    common_params params;

    if (!common_params_parse(argc, argv, params, LLAMA_EXAMPLE_LLAVA, print_usage)) {
        return 1;
    }

    common_init();

    if (params.mmproj.empty() || (params.image.empty() && !prompt_contains_image(params.prompt))) {
        print_usage(argc, argv);
        return 1;
    }

    auto * model = llava_init(&params);
    if (model == NULL) {
        fprintf(stderr, "%s: error: failed to init llava model\n", __func__);
        return 1;
    }

    if (prompt_contains_image(params.prompt)) {
        auto * ctx_llava = llava_init_context(&params, model);

        auto * image_embed = load_image(ctx_llava, &params, "");

        // process the prompt
        process_prompt(ctx_llava, image_embed, &params, params.prompt);

        llama_perf_context_print(ctx_llava->ctx_llama);
        llava_image_embed_free(image_embed);
        ctx_llava->model = NULL;
        llava_free(ctx_llava);
#ifndef NDEBUG
    } else if (params.image[0].empty()) {
        auto ctx_llava = llava_init_context(&params, model);

        debug_test_mrope_2d();
        debug_dump_img_embed(ctx_llava);

        llama_perf_context_print(ctx_llava->ctx_llama);
        ctx_llava->model = NULL;
        llava_free(ctx_llava);
#endif
    } else {
        for (auto & image : params.image) {
            auto * ctx_llava = llava_init_context(&params, model);

            auto * image_embed = load_image(ctx_llava, &params, image);
            if (!image_embed) {
                LOG_ERR("%s: failed to load image %s. Terminating\n\n", __func__, image.c_str());
                return 1;
            }

            // process the prompt
            process_prompt(ctx_llava, image_embed, &params, params.prompt);

            llama_perf_context_print(ctx_llava->ctx_llama);
            llava_image_embed_free(image_embed);
            ctx_llava->model = NULL;
            llava_free(ctx_llava);
        }
    }

    llama_free_model(model);

    return 0;
}
