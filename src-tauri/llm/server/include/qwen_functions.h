#pragma once

#include "clip.h"
#include "llama.h"
#include "llava.h"
#include "common.h"

#include "arg.h"
#include "base64.hpp"
#include "log.h"
#include "sampling.h"
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
#include <vector>
#include <string>

struct llava_context {
    struct clip_ctx* ctx_clip = NULL;
    struct llama_context* ctx_llama = NULL;
    struct llama_model* model = NULL;
};

bool qwen2vl_eval_image_embed(llama_context* ctx_llama, 
                             const struct llava_image_embed* image_embed,
                             int n_batch, int* n_past, int* st_pos_id, 
                             struct clip_image_size* image_size);

bool eval_tokens(struct llama_context* ctx_llama,
                std::vector<llama_token> tokens,
                int n_batch, int* n_past, int* st_pos_id);

bool eval_id(struct llama_context * ctx_llama, int id, 
                int * n_past, int * st_pos_id);

bool eval_string(struct llama_context* ctx_llama,
                const char* str, int n_batch,
                int* n_past, int* st_pos_id,
                bool add_bos);

const char * sample(struct common_sampler * smpl,
                           struct llama_context * ctx_llama,
                           int * n_past, int * st_pos_id);

void find_image_tag_in_prompt(const std::string& prompt, size_t& begin_out, size_t& end_out);

bool prompt_contains_image(const std::string& prompt);

llava_image_embed * llava_image_embed_make_with_prompt_base64(struct clip_ctx * ctx_clip, int n_threads, const std::string& prompt);

std::string remove_image_from_prompt(const std::string& prompt, const char * replacement = "");

void print_usage(int, char ** argv);

struct llava_image_embed * load_image(llava_context * ctx_llava, common_params * params, const std::string & fname);

void process_prompt(struct llava_context * ctx_llava, struct llava_image_embed * image_embed, common_params * params, const std::string & prompt);

struct llama_model * llava_init(common_params * params);

struct llava_context * llava_init_context(common_params * params, llama_model * model);

void llava_free(struct llava_context * ctx_llava);

#ifndef NDEBUG
void debug_test_mrope_2d();

void debug_dump_img_embed(struct llava_context * ctx_llava);
#endif