'use client';

import { MutableRefObject, useEffect } from 'react';
import gsap from 'gsap';
import { useGSAP } from '@gsap/react';
import type { HudDimensions } from '@/types/settings';

interface UseHudAnimationsArgs {
  hudDimensions: HudDimensions | null;
  inputContainerRef: MutableRefObject<HTMLDivElement | null>;
  messagesContainerRef: MutableRefObject<HTMLDivElement | null>;
  isExpanded: boolean;
  messagesLength: number;
  isStreaming: boolean;
}

export function useHudAnimations({
  hudDimensions,
  inputContainerRef,
  messagesContainerRef,
  isExpanded,
  messagesLength,
  isStreaming,
}: UseHudAnimationsArgs) {
  // Input spring entrance when HUD dimensions load
  useGSAP(() => {
    if (hudDimensions && inputContainerRef.current) {
      gsap.fromTo(
        inputContainerRef.current,
        { scale: 0, opacity: 0, transformOrigin: 'center center' },
        { scale: 1, opacity: 1, duration: 0.25, ease: 'back.out(0.8)', delay: 0.05 }
      );
    }
  }, [hudDimensions]);

  // Chat expansion / collapse animations
  useGSAP(() => {
    if (!messagesContainerRef.current) return;
    const container = messagesContainerRef.current;

    if (isExpanded && messagesLength > 0) {
      const tl = gsap.timeline();
      gsap.set(container, { padding: '12px', overflowY: 'hidden' });
      tl.to(container, {
        height: 'auto',
        opacity: 1,
        scale: 1,
        duration: 1,
        ease: 'back.out(1.2)',
        onComplete: () => {
          if (messagesContainerRef.current) gsap.set(messagesContainerRef.current, { overflowY: 'auto' });
        },
      });
      if (inputContainerRef.current) {
        tl.to(
          inputContainerRef.current,
          { y: 0, duration: 0.6, ease: 'back.out(1.2)' },
          0
        );
      }
    } else {
      gsap.to(container, {
        height: 0,
        opacity: 0,
        scale: 0.95,
        padding: '0px',
        overflowY: 'hidden',
        duration: 0.5,
        ease: 'power2.inOut',
      });
    }
  }, [isExpanded, messagesLength]);

  // Smooth height adjustments during streaming
  useEffect(() => {
    if (!messagesContainerRef.current || !isExpanded || messagesLength === 0) return;

    let animationFrame: number | null = null;
    let lastHeight = 0;

    const checkHeightChange = () => {
      const container = messagesContainerRef.current!;
      const contentDiv = container.querySelector('.flex.flex-col.space-y-2') as HTMLElement | null;
      if (!contentDiv) return;
      const newHeight = contentDiv.scrollHeight;
      if (newHeight !== lastHeight && lastHeight > 0 && isStreaming) {
        gsap.to(container, { height: newHeight + 32, duration: 0.25, ease: 'power2.out' });
      }
      lastHeight = newHeight;
      if (isStreaming) animationFrame = requestAnimationFrame(checkHeightChange);
    };

    if (isStreaming) {
      const container = messagesContainerRef.current;
      const contentDiv = container?.querySelector('.flex.flex-col.space-y-2') as HTMLElement | null;
      if (contentDiv) {
        lastHeight = contentDiv.scrollHeight;
        animationFrame = requestAnimationFrame(checkHeightChange);
      }
    }

    return () => {
      if (animationFrame) cancelAnimationFrame(animationFrame);
    };
  }, [isExpanded, messagesLength, isStreaming, messagesContainerRef]);
}

export default useHudAnimations;
