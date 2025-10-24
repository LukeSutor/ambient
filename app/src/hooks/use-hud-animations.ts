'use client';

import { RefObject, useEffect } from 'react';
import gsap from 'gsap';
import { useGSAP } from '@gsap/react';
import type { HudDimensions } from '@/types/settings';

interface UseHudAnimationsArgs {
  hudDimensions: HudDimensions | null;
  inputContainerRef: RefObject<HTMLDivElement | null>;
  messagesContainerRef: RefObject<HTMLDivElement | null>;
  isChatExpanded: boolean;
  messagesLength: number;
}

export function useHudAnimations({
  hudDimensions,
  inputContainerRef,
  messagesContainerRef,
  isChatExpanded,
  messagesLength,
}: UseHudAnimationsArgs) {
  // Input spring entrance when HUD dimensions load
  useGSAP(() => {
    if (hudDimensions && inputContainerRef.current) {
      gsap.fromTo(
        inputContainerRef.current,
        { scale: 0, opacity: 0, transformOrigin: 'center center' },
        { scale: 1, opacity: 1, duration: 0.25, ease: 'back.out(0.8)', delay: 0.1 }
      );
    }
  }, [hudDimensions]);

  // // Chat expansion / collapse animations
  // useGSAP(() => {
  //   if (!messagesContainerRef.current) return;
  //   const container = messagesContainerRef.current;

  //   if (isChatExpanded && messagesLength > 0) {
  //     const tl = gsap.timeline();
  //     gsap.set(container, { padding: '12px', overflowY: 'hidden' });
  //     tl.to(container, {
  //       opacity: 1,
  //       scale: 1,
  //       duration: 1,
  //       ease: 'back.out(1.2)',
  //       onComplete: () => {
  //         if (messagesContainerRef.current) gsap.set(messagesContainerRef.current, { overflowY: 'auto' });
  //       },
  //     });
  //     if (inputContainerRef.current) {
  //       tl.to(
  //         inputContainerRef.current,
  //         { y: 0, duration: 0.6, ease: 'back.out(1.2)' },
  //         0
  //       );
  //     }
  //   } else {
  //     gsap.to(container, {
  //       opacity: 0,
  //       scale: 0.95,
  //       overflowY: 'hidden',
  //       duration: 0.5,
  //       ease: 'power2.inOut',
  //     });
  //   }
  // }, [isChatExpanded, messagesLength]);
}

export default useHudAnimations;
