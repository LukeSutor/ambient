import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from '@tauri-apps/api/event';
import { useNavigate } from "react-router-dom";
import { Button } from "@/components/ui/button"
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card"
import { Progress } from "@/components/ui/progress"

function ModelDownloadPage() {
  const [modelDownloading, setModelDownloading] = useState(false);
  const [modelDownloadingId, setModelDownloadingId] = useState(0); // 0 means none downloading
  const [totalModelsDownloading, setTotalModelsDownloading] = useState(0);
  const [downloadSize, setDownloadSize] = useState(0);
  const [downloadProgress, setDownloadProgress] = useState(0);
  const navigate = useNavigate();

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

  async function downloadModel() {
    console.log("downloading")
    setModelDownloading(true);
    try {
        // Listen for the three types of events
        const unlisten_started = await listen('download-started', (event) => {
            // Set the model downloading number and total size
            setModelDownloadingId(event.payload.modelId);
            setTotalModelsDownloading(event.payload.totalModels);
            setDownloadSize(event.payload.contentLength);
        });
        const unlisten_progress = await listen('download-progress', (event) => {
            // Update the download progress
            setDownloadProgress(event.payload.totalProgress);
            setPreviousProgress(event.payload.totalProgress);
        });
        const unlisten_finished= await listen('download-finished', (event) => {
            setModelDownloadingId(0);
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
    async function checkModelDownload() {
      try {
        const result = await invoke("check_model_download");
        if (result) {
          navigate("/");
        }
      } catch (err) {
        console.error(`[ui] Failed to check if models are downloaded. ${err}`);
      }
    }
    checkModelDownload();
  }, [navigate]);

return (
    <div className="flex items-center justify-center w-screen h-screen">
        <Card className="w-[350px]">
            <CardHeader>
                <CardTitle>Download models</CardTitle>
                <CardDescription>Before getting started, the models must be downloaded onto your computer.</CardDescription>
            </CardHeader>
            <CardContent>
                {modelDownloading ?
                    <div className="flex flex-col items-center space-y-1 text-sm text-gray-600">
                        <div className="flex flex-row justify-between w-full">
                            <p>{modelDownloadingId === 0 ? "Contacting server..." : `Downloading model ${modelDownloadingId} of ${totalModelsDownloading}`}</p>
                            <p>{modelDownloadingId == 0 ? "0" : Math.round((downloadProgress / downloadSize) * 100)}%</p>
                        </div>
                        <Progress value={downloadProgress/downloadSize*100} />
                        <p className="w-full text-left">{modelDownloadingId === 0 ? "— of —" : `${formatBytes(downloadProgress)} of ${formatBytes(downloadSize)}`}</p>
                    </div>
                :
                    <Button onClick={downloadModel}>Download</Button>
                }
            </CardContent>
        </Card>
    </div>
);
}

export default ModelDownloadPage;
