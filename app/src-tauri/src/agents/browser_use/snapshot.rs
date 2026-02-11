//! DOM snapshot extraction JavaScript generation.
//!
//! Generates JavaScript code that extracts a compact, structured text
//! representation of all visible interactive elements on a page.
//! Uses the `browsersnapshot://` custom URL scheme for data transmission
//! back to the Rust process via navigation interception.
//!
//! The snapshot format is optimized for small LLMs:
//! - Only includes visible, interactive elements
//! - Assigns sequential numeric IDs for action targeting
//! - Compact text format (~1000-1500 tokens for a typical page)
//! - Stores element references in `window.__elements` for action execution

/// Custom URL scheme for receiving snapshot data from the WebView.
pub const SNAPSHOT_SCHEME: &str = "browsersnapshot";

/// Generate the JavaScript for DOM snapshot extraction.
///
/// This script:
/// 1. Finds all visible interactive elements
/// 2. Assigns sequential IDs and stores refs in `window.__elements`
/// 3. Builds a compact text representation
/// 4. Transmits via `browsersnapshot://` navigation chunks
///
/// # Arguments
/// * `request_id` - Unique identifier for matching responses
/// * `execution_token` - Security token preventing duplicate extractions
pub fn get_snapshot_script(request_id: &str, execution_token: &str) -> String {
    format!(
        r#"
(function() {{
    'use strict';

    const REQUEST_ID = "{request_id}";
    const EXECUTION_TOKEN = "{execution_token}";
    const CHUNK_SIZE = 1800;
    const BASE_URL = "browsersnapshot://data";
    const ERROR_URL = "browsersnapshot://error";

    // Prevent duplicate execution
    const STATE_KEY = '__snapshot_state_' + EXECUTION_TOKEN;
    if (window[STATE_KEY] !== undefined) return;
    window[STATE_KEY] = 'running';

    try {{
        const elements = [];
        const interactiveSelectors = [
            'a[href]', 'button', 'input:not([type="hidden"])',
            'select', 'textarea',
            '[role="button"]', '[role="link"]', '[role="tab"]',
            '[role="menuitem"]', '[role="checkbox"]', '[role="radio"]',
            '[role="switch"]', '[role="combobox"]', '[role="option"]',
            '[role="searchbox"]',
            '[onclick]', '[contenteditable="true"]',
            'summary'
        ];

        const seen = new Set();
        const allEls = document.querySelectorAll(interactiveSelectors.join(','));

        allEls.forEach(function(el) {{
            if (seen.has(el)) return;
            seen.add(el);

            // Skip hidden elements
            var rect = el.getBoundingClientRect();
            if (rect.width === 0 && rect.height === 0) return;

            var style = window.getComputedStyle(el);
            if (style.display === 'none' || style.visibility === 'hidden') return;
            if (parseFloat(style.opacity) === 0) return;

            // Skip elements outside viewport (with generous margin)
            if (rect.bottom < -100 || rect.top > window.innerHeight + 100) return;

            var tag = el.tagName.toLowerCase();
            var type = el.getAttribute('type') || '';
            var role = el.getAttribute('role') || '';
            var rawText = (el.textContent || '').replace(/\s+/g, ' ').trim();
            var text = rawText.substring(0, 60);
            var placeholder = el.getAttribute('placeholder') || '';
            var ariaLabel = el.getAttribute('aria-label') || '';
            var title = el.getAttribute('title') || '';
            var href = el.getAttribute('href') || '';
            var value = el.value !== undefined ? el.value : (el.getAttribute('value') || '');

            // Build description
            var desc = '';
            var label = text || ariaLabel || title;

            if (tag === 'a') {{
                var linkText = label || href.substring(0, 40);
                desc = 'link "' + linkText + '"';
            }} else if (tag === 'button' || role === 'button') {{
                desc = 'button "' + label + '"';
            }} else if (tag === 'input') {{
                var inputType = type || 'text';
                var inputLabel = ariaLabel || placeholder || label || '';
                desc = 'input[' + inputType + ']';
                if (inputLabel) desc += ' "' + inputLabel + '"';
                if (value && inputType !== 'password') desc += ' value="' + value.substring(0, 40) + '"';
                else if (placeholder && !value) desc += ' placeholder="' + placeholder.substring(0, 40) + '"';
            }} else if (tag === 'select') {{
                var selectedOpt = el.options && el.options[el.selectedIndex];
                var selectedText = selectedOpt ? selectedOpt.textContent.trim() : '';
                desc = 'select "' + (ariaLabel || label || '') + '"';
                if (selectedText) desc += ' value="' + selectedText.substring(0, 30) + '"';
            }} else if (tag === 'textarea') {{
                desc = 'textarea "' + (ariaLabel || placeholder || '') + '"';
                if (value) desc += ' value="' + value.substring(0, 40) + '"';
            }} else if (role === 'checkbox' || role === 'radio') {{
                var checked = el.checked || el.getAttribute('aria-checked') === 'true';
                desc = role + ' "' + label + '"';
                if (checked) desc += ' [checked]';
            }} else if (role === 'tab') {{
                var selected = el.getAttribute('aria-selected') === 'true';
                desc = 'tab "' + label + '"';
                if (selected) desc += ' [selected]';
            }} else {{
                desc = (role || tag) + ' "' + label + '"';
            }}

            if (el.disabled || el.getAttribute('aria-disabled') === 'true') {{
                desc += ' [disabled]';
            }}

            elements.push({{ el: el, desc: desc }});
        }});

        // Store element references for action execution
        window.__elements = elements.map(function(e) {{ return e.el; }});

        // Build output
        var url = window.location.href;
        var title = document.title || '(untitled)';
        var scrollY = Math.round(window.scrollY);
        var totalHeight = document.documentElement.scrollHeight;
        var viewportHeight = window.innerHeight;

        var output = 'URL: ' + url + '\n';
        output += 'Title: ' + title + '\n';
        output += 'Scroll: ' + scrollY + '/' + totalHeight + 'px (viewport: ' + viewportHeight + 'px)\n\n';

        if (elements.length === 0) {{
            output += '(No interactive elements found on this page)\n';
        }} else {{
            for (var i = 0; i < elements.length; i++) {{
                output += '[' + (i + 1) + '] ' + elements[i].desc + '\n';
            }}
        }}

        // Encode and send via chunked navigation
        var encoded = encodeURIComponent(output);
        var totalChunks = Math.ceil(encoded.length / CHUNK_SIZE);
        if (totalChunks === 0) totalChunks = 1;

        var chunkIndex = 0;
        function sendNextChunk() {{
            if (chunkIndex >= totalChunks) {{
                window[STATE_KEY] = 'complete';
                return;
            }}
            var start = chunkIndex * CHUNK_SIZE;
            var chunk = encoded.slice(start, start + CHUNK_SIZE);
            var encodedChunk = encodeURIComponent(chunk);
            var chunkUrl = BASE_URL + '/' + encodeURIComponent(REQUEST_ID) + '/' +
                encodeURIComponent(EXECUTION_TOKEN) + '/' +
                chunkIndex + '/' + totalChunks + '/' + encodedChunk;
            window.location.href = chunkUrl;
            chunkIndex++;
            if (chunkIndex < totalChunks) {{
                setTimeout(sendNextChunk, 10);
            }} else {{
                window[STATE_KEY] = 'complete';
            }}
        }}

        sendNextChunk();

    }} catch (e) {{
        window[STATE_KEY] = 'error';
        var errorMsg = encodeURIComponent(String(e.message || e));
        window.location.href = ERROR_URL + '/' + encodeURIComponent(REQUEST_ID) + '/' +
            encodeURIComponent(EXECUTION_TOKEN) + '/' + errorMsg;
    }}
}})();
"#
    )
}
