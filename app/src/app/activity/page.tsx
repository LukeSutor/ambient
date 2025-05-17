"use client"

import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

type Event = {
  id: number;
  timestamp: number;
  application: string;
  description: string | null;
};

const EVENTS_PER_PAGE = 20;

export default function ActivityPage() {
  const [events, setEvents] = useState<Event[]>([]);
  const [page, setPage] = useState(0);
  const [loading, setLoading] = useState(false);

  const fetchEvents = async (pageNum: number) => {
    setLoading(true);
    try {
      const offset = pageNum * EVENTS_PER_PAGE;
      const result = await invoke("get_events", {
        offset,
        limit: EVENTS_PER_PAGE,
      });
      setEvents(Array.isArray(result) ? result as Event[] : []);
    } catch (e) {
      setEvents([]);
    }
    setLoading(false);
  };

  useEffect(() => {
    fetchEvents(page);
  }, [page]);

  const handlePrev = () => setPage((p) => Math.max(0, p - 1));
  const handleNext = () => setPage((p) => p + 1);

  return (
    <div className="relative flex flex-col items-center justify-center p-4 w-full">
      <h2 className="text-xl font-bold mb-4">Activity</h2>
      {loading ? (
        <div>Loading...</div>
      ) : (
        <>
          <table className="min-w-full border mb-4">
            <thead>
              <tr>
                <th className="border px-2 py-1">Time</th>
                <th className="border px-2 py-1">Application</th>
                <th className="border px-2 py-1">Description</th>
              </tr>
            </thead>
            <tbody>
              {events.length === 0 ? (
                <tr>
                  <td colSpan={3} className="text-center py-2">No events found.</td>
                </tr>
              ) : (
                events.map((event) => (
                  <tr key={event.id}>
                    <td className="border px-2 py-1">
                      {new Date(event.timestamp * 1000).toLocaleString()}
                    </td>
                    <td className="border px-2 py-1">{event.application}</td>
                    <td className="border px-2 py-1">{event.description ?? ""}</td>
                  </tr>
                ))
              )}
            </tbody>
          </table>
          <div className="flex gap-2">
            <button
              className="px-3 py-1 border rounded disabled:opacity-50"
              onClick={handlePrev}
              disabled={page === 0}
            >
              Previous
            </button>
            <span>Page {page + 1}</span>
            <button
              className="px-3 py-1 border rounded"
              onClick={handleNext}
              disabled={events.length < EVENTS_PER_PAGE}
            >
              Next
            </button>
          </div>
        </>
      )}
    </div>
  );
}