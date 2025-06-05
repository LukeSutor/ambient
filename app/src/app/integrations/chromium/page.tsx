"use client"

import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardHeader,
  CardTitle,
  CardDescription,
  CardAction,
  CardContent,
  CardFooter,
} from "@/components/ui/card";
import {
  AlertDialog,
  AlertDialogTrigger,
  AlertDialogContent,
  AlertDialogHeader,
  AlertDialogTitle,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogCancel,
  AlertDialogAction,
} from "@/components/ui/alert-dialog";
import { Trash2, Play } from "lucide-react";

export default function Chromium() {
  const [workflows, setWorkflows] = useState<any[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [deleteId, setDeleteId] = useState<number | null>(null);
  const [deleting, setDeleting] = useState(false);

  // Fetch workflows
  const fetchWorkflows = async () => {
    setLoading(true);
    setError(null);
    try {
      const result = await invoke<any[]>("get_workflows", { offset: 0, limit: 20 });
      setWorkflows(Array.isArray(result) ? result : []);
    } catch (err: any) {
      setError("Failed to load workflows: " + (err?.message || err));
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchWorkflows();
  }, []);

  // Delete workflow
  const handleDelete = async (id: number) => {
    setDeleting(true);
    setError(null);
    try {
      await invoke("delete_workflow", { id });
      setWorkflows((prev) => prev.filter((w) => w.id !== id));
    } catch (err: any) {
      setError("Failed to delete workflow: " + (err?.message || err));
    } finally {
      setDeleting(false);
      setDeleteId(null);
    }
  };

  // Run workflow
  const handleRun = async (id: number) => {
    try {
      await invoke("run_workflow_by_id", { id });
    } catch (err: any) {
      console.error("Failed to run workflow: " + (err?.message || err));
    }
  };

  return (
    <div className="relative flex flex-col items-center justify-center p-4 w-full">
      <div className="w-full max-w-2xl">
        <h2 className="text-2xl font-bold mb-6">Browser Integration Workflows</h2>
        {loading ? (
          <div className="text-gray-500">Loading workflows...</div>
        ) : error ? (
          <div className="text-red-500">{error}</div>
        ) : workflows.length === 0 ? (
          <div className="text-gray-500">No workflows found.</div>
        ) : (
          <div className="grid gap-4">
            {workflows.map((wf) => (
              <Card key={wf.id} className="w-full">
                <CardHeader>
                  <CardTitle className="flex items-center gap-2">
                    <span>{wf.name || <span className="italic text-gray-400">Untitled</span>}</span>
                  </CardTitle>
                  <CardDescription>
                    {wf.description || <span className="italic text-gray-400">No description</span>}
                  </CardDescription>
                  <CardAction className="flex gap-2">
                    <Button
                      variant="secondary"
                      title="Run Workflow"
                      onClick={() => handleRun(wf.id)}
                    >
                      <Play className="w-4 h-4" /> Run
                    </Button>
                    <AlertDialog>
                      <AlertDialogTrigger asChild>
                        <Button
                          variant="destructive"
                          size="icon"
                          title="Delete Workflow"
                          disabled={deleting && deleteId === wf.id}
                          onClick={() => setDeleteId(wf.id)}
                        >
                          <Trash2 className="w-4 h-4" />
                        </Button>
                      </AlertDialogTrigger>
                      <AlertDialogContent>
                        <AlertDialogHeader>
                          <AlertDialogTitle>Are you absolutely sure?</AlertDialogTitle>
                          <AlertDialogDescription>
                            This action cannot be undone. This will permanently delete this workflow and remove its data from your computer.
                          </AlertDialogDescription>
                        </AlertDialogHeader>
                        <AlertDialogFooter>
                          <AlertDialogCancel onClick={() => setDeleteId(null)}>
                            Cancel
                          </AlertDialogCancel>
                          <AlertDialogAction
                            onClick={() => handleDelete(wf.id)}
                            disabled={deleting}
                          >
                            {deleting && deleteId === wf.id ? "Deleting..." : "Delete"}
                          </AlertDialogAction>
                        </AlertDialogFooter>
                      </AlertDialogContent>
                    </AlertDialog>
                  </CardAction>
                </CardHeader>
                <CardContent>
                  <div className="text-xs text-gray-600">
                    <b>URL:</b> {wf.url ?? <span className="italic text-gray-400">none</span>}<br />
                    <b>Steps:</b> {(() => { try { return JSON.parse(wf.steps_json ?? "[]").length; } catch { return 0; } })()}<br />
                    <b>Recorded:</b> {typeof wf.recording_start === "number" && wf.recording_start > 0
                      ? (() => {
                          const ts = wf.recording_start > 1e12 ? Math.floor(wf.recording_start / 1000) : wf.recording_start;
                          return new Date(ts * 1000).toLocaleString();
                        })()
                      : "?"}
                    {" - "}
                    {typeof wf.recording_end === "number" && wf.recording_end > 0
                      ? (() => {
                          const ts = wf.recording_end > 1e12 ? Math.floor(wf.recording_end / 1000) : wf.recording_end;
                          return new Date(ts * 1000).toLocaleString();
                        })()
                      : "?"}
                  </div>
                </CardContent>
              </Card>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}