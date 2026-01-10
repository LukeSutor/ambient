"use client";

import React, { useEffect, useState } from 'react';
import Image from 'next/image';
import { invoke } from '@tauri-apps/api/core';
import { listen, UnlistenFn } from '@tauri-apps/api/event';
import { ContentContainer } from '@/components/hud/content-container';
import { ComputerUseToastEvent } from '@/types/events';
import { Button } from '@/components/ui/button';
import { X } from 'lucide-react';

const startupToasts = [
	"Dusting off keyboard",
	"Warming up pixels",
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

	const getRandomToast = () => startupToasts[Math.floor(Math.random() * startupToasts.length)];

	useEffect(() => {
		setToastMessage(getRandomToast());

		let unlisten: UnlistenFn = () => {};
		const setupListener = async () => {
			// Listen for computer use toast events to update text
			const unlistenFn = await listen<ComputerUseToastEvent>('computer_use_toast', (event) => {
				console.log('Received computer use toast event:', event.payload);
				setToastMessage(event.payload.message);
			});
			unlisten = unlistenFn;
		}

		setupListener();

		return () => {
			if (unlisten) unlisten();
		};
	}, [])

	const closeToast = async () => {
		await invoke('close_computer_use_window');
	}

  return (
    <>
			<style
				// eslint-disable-next-line react/no-danger
				dangerouslySetInnerHTML={{
					__html:
					"html,body{background:transparent!important;background-color:transparent!important;height:100%!important;width:100%!important;overflow:hidden!important;}}",
				}}
			/>
			<ContentContainer className="flex items-center h-full">
				<div className="flex flex-row space-x-2 p-2 w-full">
					<Image
						src="/logo.png"
						alt="Computer Use Icon"
						width={24}
						height={24}
					/>
					<p>{toastMessage}
						<span className="ml-[6px] inline-flex">
							<span className=" animate-bounce size-2 -ml-[3px] ">.</span>
							<span className=" animate-bounce size-2 -ml-[3px] [animation-delay:200ms]">.</span>
							<span className=" animate-bounce size-2 -ml-[3px] [animation-delay:400ms]">.</span>
						</span>
					</p>
				</div>
				<Button size="icon" variant="ghost" className="mr-1 rounded-full w-7 h-7" onClick={closeToast}>
					<X className="w-4 h-4" />
				</Button>
			</ContentContainer>
    </>
  );
}

export default ComputerUsePage;