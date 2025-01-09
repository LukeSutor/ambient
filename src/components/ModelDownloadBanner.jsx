import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from '@tauri-apps/api/event';

function ModelDownloadBanner() {
    const [modelDownloaded, setModelDownloaded] = useState(true); // Default to the model being downloaded
    const [modelDownloading, setModelDownloading] = useState(false);
    const [modelDownloadingName, setModelDownloadingName] = useState("");
    const [downloadSize, setDownloadSize] = useState(0);
    const [downloadProgress, setDownloadProgress] = useState(0);

    function formatBytes(bytes) {
        if (bytes === 0) return '0 Bytes';
        const k = 1024;
        const sizes = ['Bytes', 'KB', 'MB', 'GB', 'TB'];
        const i = Math.floor(Math.log(bytes) / Math.log(k));
        const size = bytes / Math.pow(k, i);
        if (i === 3 && size < 10) {
            return `${size.toFixed(2)}${sizes[i]}`;
        } else if (size < 10) {
            return `${size.toFixed(1)}${sizes[i]}`;
        } else {
            return `${Math.floor(size)}${sizes[i]}`;
        }
    }

    async function checkModelDownload() {
        // Check if model is downloaded
        try {
            const result = await invoke("check_model_download");
            console.log("Download check result:", result);
            setModelDownloaded(result);
            return;
        } catch (err) {
            console.error(`[ui] Failed to check if models are downloaded. ${err}`);
        }
    }

    async function downloadModel() {
        console.log("downloading")
        setModelDownloading(true);
        try {
            // Listen for the three types of events
            const unlisten_started = await listen('download-started', (event) => {
                // Set the model downloading number and total size
                setModelDownloadingName(event.payload.modelName);
                setDownloadSize(event.payload.contentLength);
            });
            const unlisten_progress = await listen('download-progress', (event) => {
                // Update the download progress
                setDownloadProgress(event.payload.totalProgress);
            });
            const unlisten_finished= await listen('download-finished', (event) => {
                setModelDownloadingName("");
            });
            await invoke("download_model");
            unlisten_started();
            unlisten_progress();
            unlisten_finished();
            return;
        } catch (err) {
            console.error(`[ui] Failed to check if models are downloaded. ${err}`);
        }
        setModelDownloading(false);
    }

    useEffect(() => {
        checkModelDownload();
        // Re-check model download status at every change of model downloading
    }, [modelDownloading]);

    if (modelDownloaded) {
        return (<></>);
    }

    return (
        <>
            {modelDownloading ?
                <div className="">
                    <div className="w-full h-2">
                        <div className="bg-blue-500 h-full" style={{width: `${modelDownloadingName === "" ? 0 : (downloadProgress / downloadSize) * 100}%`}} />
                    </div>
                    <div className="flex flex-row space-x-4 items-center justify-center">
                        {modelDownloadingName === "" ?
                            <p className="text-center font-semibold">Contacting server</p>
                            :
                            <p className="text-center font-semibold">Downloading {modelDownloadingName} <span className="text-gray-600 font-normal">({formatBytes(downloadProgress)} of {formatBytes(downloadSize)})</span></p>
                        }
                        <div className="spinner-border animate-spin inline-block w-4 h-4 border-2 rounded-full border-blue-600 border-t-transparent"></div>
                    </div>
                </div>
                :
                <div className="bg-blue-600 text-white flex flex-row items-center justify-center w-full py-1 px-4 space-x-2">
                    <p className="text-center">Notice: models not downloaded.</p>
                    <button onClick={downloadModel} className="font-bold underline">DOWNLOAD NOW</button>
                </div>
            }
        </>
    );
}

export default ModelDownloadBanner;