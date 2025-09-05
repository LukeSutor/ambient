'use client';

import React from 'react';

// Lightweight text renderer for streaming content without heavy markdown costs
export function AnimatedText({ content }: { content: string }) {
  return <div className="whitespace-pre-wrap">{content}</div>;
}

export default AnimatedText;
