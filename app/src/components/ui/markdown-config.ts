import { Options } from 'react-markdown';
import remarkGfm from 'remark-gfm';
import rehypeKatex from 'rehype-katex'
import remarkMath from 'remark-math'
import { markdownComponents, darkMarkdownComponents, customUrlTransform } from './markdown-components';

// Basic configuration for LLM-generated content
export const basicMarkdownConfig: Options = {
  remarkPlugins: [[remarkGfm, { singleTilde: false }], [remarkMath, { singleDollarTextMath: false }]],
  rehypePlugins: [[rehypeKatex, { output: 'html' }]],
  components: markdownComponents,
  urlTransform: customUrlTransform,
};

// Dark theme configuration
export const darkMarkdownConfig: Options = {
  remarkPlugins: [[remarkGfm, { singleTilde: false }], [remarkMath, { singleDollarTextMath: false }]],
  rehypePlugins: [[rehypeKatex, { output: 'html' }]],
  components: darkMarkdownComponents,
  urlTransform: customUrlTransform,
};

// Strict security configuration (for untrusted content)
export const strictSecureMarkdownConfig: Options = {
  remarkPlugins: [[remarkGfm, { singleTilde: false }], [remarkMath, { singleDollarTextMath: false }]],
  rehypePlugins: [[rehypeKatex, { output: 'html' }]],
  components: markdownComponents,
  urlTransform: customUrlTransform,
  // Disallow potentially dangerous elements
  disallowedElements: ['script', 'style', 'iframe', 'object', 'embed'],
  // Skip HTML entirely for maximum security
  skipHtml: true,
};

// Export commonly used configs
export const llmMarkdownConfig = basicMarkdownConfig;
export const secureMarkdownConfig = strictSecureMarkdownConfig;
