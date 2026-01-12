import { Options } from 'react-markdown';
import remarkGfm from 'remark-gfm';
import rehypeKatex from 'rehype-katex'
import remarkMath from 'remark-math'
import { markdownComponents, darkMarkdownComponents, customUrlTransform } from './markdown-components';

/**
 * Preprocesses markdown content to escape currency dollar signs while preserving math.
 * 
 * The LLM outputs math with spaces like "$ F=ma $" but currency without spaces like "$100".
 * This function escapes $ signs that are immediately followed by digits (currency),
 * leaving spaced math expressions intact for remark-math to process.
 */
export function preprocessMarkdownCurrency(content: string): string {
  // Escape $ when immediately followed by a digit (currency like $100, $1,000, $1.50)
  // This regex matches $ followed by a digit, and replaces $ with \$
  return content.replace(/\$(?=\d)/g, '\\$');
}

// Basic configuration for LLM-generated content
export const basicMarkdownConfig: Options = {
  remarkPlugins: [[remarkGfm, { singleTilde: false }], [remarkMath]],
  rehypePlugins: [[rehypeKatex, { output: 'html' }]],
  components: markdownComponents,
  urlTransform: customUrlTransform,
};

// Dark theme configuration
export const darkMarkdownConfig: Options = {
  remarkPlugins: [[remarkGfm, { singleTilde: false }], [remarkMath]],
  rehypePlugins: [[rehypeKatex, { output: 'html' }]],
  components: darkMarkdownComponents,
  urlTransform: customUrlTransform,
};

// Strict security configuration (for untrusted content)
export const strictSecureMarkdownConfig: Options = {
  remarkPlugins: [[remarkGfm, { singleTilde: false }], [remarkMath]],
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
