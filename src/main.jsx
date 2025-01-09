import React from "react";
import ReactDOM from "react-dom/client";
import { BrowserRouter as Router, Route, Routes, Navigate } from "react-router-dom";
import "./index.css";
import App from "./App";
import ModelDownloadPage from "./pages/ModelDownloadPage";
import Debug from "./pages/Debug";

ReactDOM.createRoot(document.getElementById("root")).render(
  <React.StrictMode>
    <Router>
      <Routes>
        <Route path="/download" element={<ModelDownloadPage />} />
        <Route path="/debug" element={<Debug />} />
        <Route path="/" element={<App />} />
      </Routes>
    </Router>
  </React.StrictMode>,
);
