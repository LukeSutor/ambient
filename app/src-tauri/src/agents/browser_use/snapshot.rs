//! DOM snapshot extraction JavaScript generation.
//!
//! Generates JavaScript that performs a single document-order DOM walk,
//! producing a markdown-formatted snapshot with interactive element IDs
//! inline alongside page content. This interleaved format means the model
//! sees elements in context (e.g., a product name, price, and "Add to cart"
//! button together) rather than in disconnected sections.
//!
//! Key features:
//! - Single-pass DOM walker with markdown formatting (headings, lists, tables)
//! - Interactive elements inline with `@id` notation for clear reference
//! - Image alt text included for visual context
//! - Navigation/footer noise filtered to only form inputs (search bars)
//! - Viewport-bounded with ±200px buffer
//! - Stores element references in `window.__elements` for action execution
//! - Chunked transmission via `browsersnapshot://` custom URL scheme

/// Custom URL scheme for receiving snapshot data from the WebView.
pub const SNAPSHOT_SCHEME: &str = "browsersnapshot";

/// Maximum total characters for the interleaved snapshot output.
/// ~2000-2500 tokens; covers both content and elements in a single stream.
const MAX_SNAPSHOT_CHARS: usize = 8000;

/// Generate the JavaScript for DOM snapshot extraction.
///
/// Performs a single document-order DOM walk that:
/// 1. Produces markdown-formatted text content
/// 2. Includes interactive elements inline with `@id` notation
/// 3. Includes image alt text for visual context
/// 4. Filters nav/footer noise (only form inputs kept)
/// 5. Stores element refs in `window.__elements` for action execution
/// 6. Transmits via `browsersnapshot://` navigation chunks
///
/// # Arguments
/// * `request_id` - Unique identifier for matching responses
/// * `execution_token` - Security token preventing duplicate extractions
pub fn get_snapshot_script(request_id: &str, execution_token: &str) -> String {
    format!(
        r#"
(function() {{
    'use strict';

    var REQUEST_ID = "{request_id}";
    var EXECUTION_TOKEN = "{execution_token}";
    var CHUNK_SIZE = 1800;
    var MAX_CHARS = {max_chars};
    var BASE_URL = "browsersnapshot://data";
    var ERROR_URL = "browsersnapshot://error";
    var STATE_KEY = '__snapshot_state_' + EXECUTION_TOKEN;
    if (window[STATE_KEY] !== undefined) return;
    window[STATE_KEY] = 'running';

    try {{
        var els = [];  // Element refs for action execution (1-based ID = index + 1)
        var out = [];  // Output text parts
        var len = 0;   // Running character count
        var VH = window.innerHeight;
        var SKIP = {{'SCRIPT':1,'STYLE':1,'NOSCRIPT':1,'SVG':1,'HEAD':1,'META':1,'LINK':1,'TEMPLATE':1}};

        // ================================================================
        // Helpers
        // ================================================================

        // Append text to output, respecting character limit
        function add(s) {{
            if (len >= MAX_CHARS) return false;
            out.push(s);
            len += s.length;
            return len < MAX_CHARS;
        }}

        // Register an interactive element, return its 1-based ID
        function reg(el) {{
            els.push(el);
            return els.length;
        }}

        // Get a human-readable label for an element.
        // Checks aria-label, title, child img alt text, then textContent.
        function lbl(el) {{
            var a = el.getAttribute('aria-label');
            if (a) return a.replace(/\s+/g, ' ').trim().substring(0, 60);
            var t = el.getAttribute('title');
            if (t) return t.replace(/\s+/g, ' ').trim().substring(0, 60);
            // Check for child images (for links wrapping images)
            var imgs = el.getElementsByTagName('img');
            var imgAlt = '';
            for (var j = 0; j < imgs.length; j++) {{
                var ia = imgs[j].getAttribute('alt') || imgs[j].getAttribute('title') || '';
                if (ia && ia.trim().length > 1) {{ imgAlt = ia.trim(); break; }}
            }}
            var raw = (el.textContent || '').replace(/\s+/g, ' ').trim();
            if (imgAlt && !raw) return imgAlt.substring(0, 60);
            if (imgAlt && raw.length < 3) return imgAlt.substring(0, 60);
            return raw.substring(0, 60) || '';
        }}

        // Check element visibility (display, opacity, viewport proximity)
        function vis(el) {{
            try {{
                var r = el.getBoundingClientRect();
                if (r.width === 0 && r.height === 0) return false;
                if (r.bottom < -200 || r.top > VH + 200) return false;
                var s = window.getComputedStyle(el);
                if (s.display === 'none' || s.visibility === 'hidden') return false;
                if (parseFloat(s.opacity) === 0) return false;
                return true;
            }} catch(e) {{ return false; }}
        }}

        // ================================================================
        // Main recursive DOM walker
        // navMode: true inside <nav>/<footer> — only extract form inputs
        // ================================================================
        function walk(node, navMode) {{
            if (len >= MAX_CHARS) return;

            // --- Text node ---
            if (node.nodeType === 3) {{
                if (navMode) return;
                var text = node.textContent;
                if (!text || !text.trim()) return;
                text = text.replace(/\s+/g, ' ');
                // Viewport check via parent element
                var p = node.parentElement;
                if (p) {{
                    try {{
                        var r = p.getBoundingClientRect();
                        if (r.bottom < -200 || r.top > VH + 200) return;
                        if (r.width === 0 && r.height === 0) return;
                    }} catch(e) {{}}
                }}
                add(text);
                return;
            }}

            if (node.nodeType !== 1) return;

            var el = node;
            var tag = el.tagName;
            if (SKIP[tag]) return;
            if (!vis(el)) return;

            var t = tag.toLowerCase();
            var role = el.getAttribute('role') || '';

            // =============================================================
            // Navigation & footer: reduced mode (only form inputs)
            // =============================================================
            if (t === 'nav' || t === 'footer') {{
                var inputs = el.querySelectorAll('input:not([type="hidden"]), textarea, select');
                for (var i = 0; i < inputs.length; i++) {{
                    var inp = inputs[i];
                    if (!vis(inp)) continue;
                    var id = reg(inp);
                    var iT = inp.type || 'text';
                    var iL = inp.getAttribute('aria-label') || inp.getAttribute('placeholder') || '';
                    var d = '[in(' + iT + '): ' + iL.substring(0, 30);
                    if (inp.value && iT !== 'password') d += ' = "' + inp.value.substring(0, 30) + '"';
                    d += ' @' + id + '] ';
                    add(d);
                }}
                return;
            }}

            // =============================================================
            // Headings
            // =============================================================
            var hLvl = 0;
            if (t === 'h1') hLvl = 1;
            else if (t === 'h2') hLvl = 2;
            else if (t === 'h3') hLvl = 3;
            else if (t === 'h4' || t === 'h5' || t === 'h6') hLvl = 4;
            else if (role === 'heading') hLvl = parseInt(el.getAttribute('aria-level') || '3');

            if (hLvl > 0) {{
                var hText = lbl(el);
                if (hText) add('\n' + '#'.repeat(Math.min(hLvl, 4)) + ' ' + hText + '\n');
                return;
            }}

            // =============================================================
            // Images — include alt text for visual context
            // =============================================================
            if (t === 'img') {{
                var alt = el.getAttribute('alt') || el.getAttribute('title') || el.getAttribute('aria-label') || '';
                alt = alt.trim();
                if (alt && alt.length > 1) {{
                    add('[img: ' + alt.substring(0, 80) + '] ');
                }}
                return;
            }}

            // =============================================================
            // Checkbox / Radio (native + ARIA) — before generic input
            // =============================================================
            if ((t === 'input' && (el.type === 'checkbox' || el.type === 'radio')) ||
                role === 'checkbox' || role === 'radio') {{
                if (navMode) return;
                var id = reg(el);
                var ck = el.getAttribute('aria-checked') === 'true' || el.checked;
                var cLbl = lbl(el);
                if (!cLbl && el.id) {{
                    var lFor = document.querySelector('label[for="' + el.id + '"]');
                    if (lFor) cLbl = lFor.textContent.replace(/\s+/g,' ').trim().substring(0, 30);
                }}
                add('[' + (ck ? 'x' : ' ') + '] ' + (cLbl || '').substring(0, 30) + ' @' + id + ' ');
                return;
            }}

            // =============================================================
            // Links
            // =============================================================
            if (t === 'a' && el.hasAttribute('href')) {{
                if (navMode) return;
                var id = reg(el);
                var lt = lbl(el);
                if (!lt) return;
                add('[' + lt + ' @' + id + '] ');
                return;
            }}

            // =============================================================
            // Buttons (native + role="button")
            // =============================================================
            if (t === 'button' || (role === 'button' && t !== 'a' && t !== 'input')) {{
                if (navMode) return;
                var id = reg(el);
                var bt = lbl(el);
                var dis = el.disabled || el.getAttribute('aria-disabled') === 'true';
                var d = '[btn: ' + (bt || '?').substring(0, 40) + ' @' + id;
                if (dis) d += ' disabled';
                d += '] ';
                add(d);
                return;
            }}

            // =============================================================
            // Inputs (generic — checkbox/radio handled above)
            // =============================================================
            if (t === 'input' && el.type !== 'hidden') {{
                var id = reg(el);
                var iType = el.type || 'text';
                var iLabel = el.getAttribute('aria-label') || el.getAttribute('placeholder') || '';
                var d = '[in(' + iType + '): ' + iLabel.substring(0, 30);
                if (el.value && iType !== 'password') d += ' = "' + el.value.substring(0, 30) + '"';
                d += ' @' + id + '] ';
                add(d);
                return;
            }}

            // =============================================================
            // Select dropdowns
            // =============================================================
            if (t === 'select') {{
                var id = reg(el);
                var sL = el.getAttribute('aria-label') || '';
                var sV = el.options && el.options[el.selectedIndex]
                    ? el.options[el.selectedIndex].textContent.trim() : '';
                add('[sel: ' + sL.substring(0, 25) + ' = "' + sV.substring(0, 20) + '" @' + id + '] ');
                return;
            }}

            // =============================================================
            // Textarea
            // =============================================================
            if (t === 'textarea') {{
                var id = reg(el);
                var tL = el.getAttribute('aria-label') || el.getAttribute('placeholder') || '';
                add('[txt: ' + tL.substring(0, 30) + ' @' + id + '] ');
                return;
            }}

            // =============================================================
            // Summary (toggle for <details>)
            // =============================================================
            if (t === 'summary') {{
                if (navMode) return;
                var id = reg(el);
                add('[btn: ' + lbl(el).substring(0, 40) + ' @' + id + '] ');
                return;
            }}

            // =============================================================
            // ARIA: tabs
            // =============================================================
            if (role === 'tab') {{
                if (navMode) return;
                var id = reg(el);
                var sel = el.getAttribute('aria-selected') === 'true';
                add('[tab: ' + lbl(el).substring(0, 30) + (sel ? '*' : '') + ' @' + id + '] ');
                return;
            }}

            // =============================================================
            // ARIA: other interactive roles
            // =============================================================
            var interactiveRoles = {{'menuitem':1,'option':1,'switch':1,'combobox':1,'searchbox':1,'treeitem':1}};
            if (interactiveRoles[role]) {{
                if (navMode) return;
                var id = reg(el);
                add('[' + lbl(el).substring(0, 40) + ' @' + id + '] ');
                return;
            }}

            // =============================================================
            // Elements with onclick (treat as buttons if not already handled)
            // =============================================================
            if (!navMode && el.hasAttribute('onclick') &&
                !{{'A':1,'BUTTON':1,'INPUT':1,'SELECT':1,'TEXTAREA':1}}[tag]) {{
                var ot = lbl(el);
                if (ot && ot.length < 60) {{
                    var id = reg(el);
                    add('[btn: ' + ot.substring(0, 40) + ' @' + id + '] ');
                    return;
                }}
            }}

            // =============================================================
            // Contenteditable (treat as text input)
            // =============================================================
            if (el.getAttribute('contenteditable') === 'true') {{
                var id = reg(el);
                var ct = el.textContent.trim().substring(0, 30);
                add('[edit: ' + (ct || 'editable') + ' @' + id + '] ');
                return;
            }}

            // =============================================================
            // STRUCTURAL ELEMENTS
            // =============================================================

            // --- Lists ---
            if (t === 'ul' || t === 'ol' || role === 'list') {{
                add('\n');
                var items = el.children;
                for (var i = 0; i < items.length; i++) {{
                    var child = items[i];
                    var ct2 = child.tagName ? child.tagName.toLowerCase() : '';
                    if (ct2 === 'li' || (child.getAttribute && child.getAttribute('role') === 'listitem')) {{
                        add('- ');
                        walk(child, navMode);
                        add('\n');
                    }} else {{
                        walk(child, navMode);
                    }}
                }}
                return;
            }}

            // --- Tables ---
            if (t === 'table') {{
                var rows = el.querySelectorAll('tr');
                if (rows.length > 0 && rows.length < 50) {{
                    add('\n');
                    var hdDone = false;
                    for (var ri = 0; ri < Math.min(rows.length, 25); ri++) {{
                        var cells = rows[ri].querySelectorAll('th, td');
                        var rp = [];
                        cells.forEach(function(c) {{
                            rp.push(c.textContent.replace(/\s+/g, ' ').trim().substring(0, 40));
                        }});
                        if (rp.length > 0) {{
                            add('| ' + rp.join(' | ') + ' |\n');
                            if (!hdDone && rows[ri].querySelector('th')) {{
                                add('|' + rp.map(function() {{ return '---'; }}).join('|') + '|\n');
                                hdDone = true;
                            }}
                        }}
                    }}
                    add('\n');
                }}
                return;
            }}

            // --- Paragraphs ---
            if (t === 'p') {{
                add('\n');
                for (var i = 0; i < el.childNodes.length; i++) walk(el.childNodes[i], navMode);
                add('\n');
                return;
            }}

            // --- Line breaks ---
            if (t === 'br') {{ add('\n'); return; }}
            if (t === 'hr') {{ add('\n---\n'); return; }}

            // --- Bold / Emphasis ---
            if (t === 'strong' || t === 'b') {{
                add('**');
                for (var i = 0; i < el.childNodes.length; i++) walk(el.childNodes[i], navMode);
                add('** ');
                return;
            }}
            if (t === 'em' || t === 'i') {{
                add('*');
                for (var i = 0; i < el.childNodes.length; i++) walk(el.childNodes[i], navMode);
                add('* ');
                return;
            }}

            // --- Generic block containers ---
            var display = window.getComputedStyle(el).display;
            var isBlock = display === 'block' || display === 'flex' || display === 'grid' ||
                          display === 'list-item' || display === 'table';

            if (isBlock) add('\n');

            for (var i = 0; i < el.childNodes.length; i++) {{
                walk(el.childNodes[i], navMode);
            }}

            if (isBlock) add('\n');
        }}

        // ================================================================
        // Build page header and run walker
        // ================================================================
        var url = window.location.href;
        var title = document.title || '(untitled)';
        var scrollY = Math.round(window.scrollY);
        var totalH = document.documentElement.scrollHeight;

        add('URL: ' + url + '\n');
        add('Title: ' + title + '\n');
        add('Scroll: ' + scrollY + '/' + totalH + 'px (viewport: ' + VH + 'px)\n\n');

        // Walk DOM from body
        walk(document.body || document.documentElement, false);

        // Store element refs for action execution
        window.__elements = els;

        // Clean up output: collapse whitespace, limit blank lines
        var result = out.join('')
            .replace(/[ \t]+/g, ' ')
            .replace(/\n /g, '\n')
            .replace(/\n{{3,}}/g, '\n\n')
            .trim();

        // ================================================================
        // Chunked transmission via browsersnapshot:// navigation
        // ================================================================
        var encoded = encodeURIComponent(result);
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
"#,
        max_chars = MAX_SNAPSHOT_CHARS
    )
}
