"use client";

import React, { useEffect, useState, useRef } from 'react';
import Image from 'next/image';
import { invoke } from '@tauri-apps/api/core';
import { listen, UnlistenFn, emit } from '@tauri-apps/api/event';
import { ContentContainer } from '@/components/hud/content-container';
import { ComputerUseToastEvent, SafetyConfirmationEvent, SafetyConfirmationResponseEvent } from '@/types/events';
import { Button } from '@/components/ui/button';
import { ArrowUpRight, X } from 'lucide-react';
import {
  Empty,
  EmptyContent,
  EmptyDescription,
  EmptyHeader,
  EmptyMedia,
  EmptyTitle,
} from "@/components/ui/empty"

const startupToasts = [
	"Dusting off the keyboard",
	"Warming up the pixels",
	"Feeding the hamsters",
	"Polishing the bits",
	"Untangling cables",
	"Charging flux capacitor",
	"Bribing the CPU",
	"Waking up the electrons",
	"Caffeinating servers",
	"Petting the mouse",
	"Defragmenting vibes",
	"Spinning up hamster wheel",
	"Reticulating splines",
	"Counting to infinity",
	"Blowing on cartridge",
	"Consulting the manual",
	"Aligning bits",
	"Tickling transistors",
	"Summoning digital spirits",
	"Eating bytes"
];

function ComputerUsePage() {
	const [toastMessage, setToastMessage] = useState<string>();
	const [confirmationRequired, setConfirmationRequired] = useState<boolean>(false);
	const [confirmationMessage, setConfirmationMessage] = useState<string>("");
	const containerRef = useRef<HTMLDivElement>(null);

	const getRandomToast = () => startupToasts[Math.floor(Math.random() * startupToasts.length)];

	useEffect(() => {
		const container = containerRef.current;
		if (!container) return;

		const resizeWindow = async () => {
			const rect = container.getBoundingClientRect();
			const width = Math.ceil(rect.width);
			const height = Math.ceil(rect.height);

			try {
				await invoke('resize_computer_use_window', { width, height });
			} catch (error) {
				console.error('[ComputerUsePage] Failed to resize window:', error);
			}
		};

		const observer = new ResizeObserver(() => {
			resizeWindow();
		});

		observer.observe(container);
		resizeWindow();

		return () => observer.disconnect();
	}, []);

	useEffect(() => {
		setToastMessage(getRandomToast());

		// Listen for computer use toast and safety confirmation events
		let unlisteners: UnlistenFn[] = [];
		const setupListeners = async () => {
			// Listen for computer use toast events to update text
			const computerUseUnlisten = await listen<ComputerUseToastEvent>('computer_use_toast', (event) => {
				setToastMessage(event.payload.message);
			});
			unlisteners.push(computerUseUnlisten);

			// Listen for safety confirmation events
			const safetyConfirmUnlisten = await listen<SafetyConfirmationEvent>('get_safety_confirmation', (event) => {
				console.log('Received safety confirmation event:', event);
				setConfirmationMessage(event.payload.reason);
				setConfirmationRequired(true);
			});
			unlisteners.push(safetyConfirmUnlisten);
		}

		setupListeners();

		return () => {
			unlisteners.forEach(unlisten => unlisten());
		};
	}, [])

	const closeToast = async () => {
		await invoke('close_computer_use_window');
	}

	const handleConfirmation = async (confirmed: boolean) => {
		let event: SafetyConfirmationResponseEvent = {
			user_confirmed: confirmed,
			timestamp: Date.now().toString()
		}
		await emit('safety_confirmation_response', event);
		setConfirmationRequired(false);
		setConfirmationMessage("");
		setToastMessage("Processing confirmation");
	}

	const openMainWindow = async () => {
		try {
			await invoke('open_main_window');
		} catch (error) {
			console.error('Failed to open main window:', error);
		}
	}

  return (
    <div ref={containerRef} className="w-fit h-fit overflow-hidden">
			<style
				// eslint-disable-next-line react/no-danger
				dangerouslySetInnerHTML={{
					__html:
					"html,body{background:transparent!important;background-color:transparent!important;overflow:hidden!important;}}",
				}}
			/>
			<ContentContainer className="flex flex-col items-center min-w-[300px] whitespace-nowrap overflow-hidden">
				{confirmationRequired ? (
					<div className="overflow-hidden">
						<Empty>
							<EmptyHeader>
								<EmptyMedia>
									{/* <ShieldAlert /> */}
									<Image src="/logo.png" width={32} height={32} alt="Safety Icon" />
								</EmptyMedia>
								<EmptyTitle>Safety Confirmation</EmptyTitle>
								<EmptyDescription className="text-gray-600">Your confirmation is necessary to proceed.{confirmationMessage && " Reasoning:"}</EmptyDescription>
								{confirmationMessage && <EmptyDescription className="text-gray-600">{confirmationMessage}</EmptyDescription>}
							</EmptyHeader>
							<EmptyContent>
								<div className="flex flex-row space-x-2 mt-4">
									<Button variant="outline" onClick={() => handleConfirmation(false)}>Cancel</Button>
									<Button className="bg-blue-500 hover:bg-blue-600" onClick={() => handleConfirmation(true)}>Confirm</Button>
								</div>
							</EmptyContent>
							<Button variant="link" onClick={openMainWindow}>
								See model steps <ArrowUpRight />
							</Button>
						</Empty>
					</div>
				) : (
					<div className="flex flex-row items-center space-x-2 p-2 w-full">
						<Image
							src="/logo.png"
							alt="Computer Use Icon"
							width={24}
							height={24}
						/>
						<p className="flex items-center w-full">{toastMessage}
							<span className="ml-[5px] inline-flex mb-3 flex-shrink-0">
								<span className="animate-bounce size-2 -ml-[3px] ">.</span>
								<span className="animate-bounce size-2 -ml-[3px] [animation-delay:200ms]">.</span>
								<span className="animate-bounce size-2 -ml-[3.5px] [animation-delay:400ms]">.</span>
							</span>
						</p>
						<Button size="icon" variant="ghost" className="ml-2 rounded-full w-7 h-7 shrink-0" onClick={closeToast}>
							<X className="w-4 h-4" />
						</Button>
					</div>
				)}
			</ContentContainer>
    </div>
  );
}

export default ComputerUsePage;