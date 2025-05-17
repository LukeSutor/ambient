// This script gets injected into any opened page
// whose URL matches the pattern defined in the manifest
// (see "content_script" key).
// Several foreground scripts can be declared
// and injected into the same or different pages.

console.log("This prints to the console of the page (injected only if the page url matched)")

// --- Helper function to generate XPath ---
function getXPath(element) {
  if (element.id !== '') {
    return `id("${element.id}")`;
  }
  if (element === document.body) {
    return element.tagName.toLowerCase();
  }

  let ix = 0;
  const siblings = element.parentNode?.children;
  if (siblings) {
    for (let i = 0; i < siblings.length; i++) {
      const sibling = siblings[i];
      if (sibling === element) {
        return `${getXPath(
          element.parentElement
        )}/${element.tagName.toLowerCase()}[${ix + 1}]`;
      }
      if (sibling.nodeType === 1 && sibling.tagName === element.tagName) {
        ix++;
      }
    }
  }
  // Fallback (should not happen often)
  return element.tagName.toLowerCase();
}

// --- Helper function to generate CSS Selector ---
// Expanded set of safe attributes (similar to Python)
const SAFE_ATTRIBUTES = new Set([
  'id',
  'name',
  'type',
  'placeholder',
  'aria-label',
  'aria-labelledby',
  'aria-describedby',
  'role',
  'for',
  'autocomplete',
  'required',
  'readonly',
  'alt',
  'title',
  'src',
  'href',
  'target',
  // Add common data attributes if stable
  'data-id',
  'data-qa',
  'data-cy',
  'data-testid',
]);

function getEnhancedCSSSelector(element, xpath) {
  try {
    // Base selector from simplified XPath or just tagName
    let cssSelector = element.tagName.toLowerCase();

    // Handle class attributes
    if (element.classList && element.classList.length > 0) {
      const validClassPattern = /^[a-zA-Z_][a-zA-Z0-9_-]*$/;
      element.classList.forEach((className) => {
        if (className && validClassPattern.test(className)) {
          cssSelector += `.${CSS.escape(className)}`;
        }
      });
    }

    // Handle other safe attributes
    for (const attr of element.attributes) {
      const attrName = attr.name;
      const attrValue = attr.value;

      if (attrName === 'class') continue;
      if (!attrName.trim()) continue;
      if (!SAFE_ATTRIBUTES.has(attrName)) continue;

      const safeAttribute = CSS.escape(attrName);

      if (attrValue === '') {
        cssSelector += `[${safeAttribute}]`;
      } else {
        const safeValue = attrValue.replace(/"/g, '"');
        if (/["'<>`\s]/.test(attrValue)) {
          cssSelector += `[${safeAttribute}*="${safeValue}"]`;
        } else {
          cssSelector += `[${safeAttribute}="${safeValue}"]`;
        }
      }
    }
    return cssSelector;
  } catch (error) {
    console.error('Error generating enhanced CSS selector:', error);
    return `${element.tagName.toLowerCase()}[xpath="${xpath.replace(
      /"/g,
      '"'
    )}"]`;
  }
}

// Minimal WebSocket client that tries a range of ports to connect to the Tauri server
let ws = null;
let connectedPort = null;
const PORT_RANGE = Array.from({length: 11}, (_, i) => 3010 + i);

async function connectToTauriWebSocket() {
    for (const port of PORT_RANGE) {
        try {
            const socket = new WebSocket(`ws://127.0.0.1:${port}/ws`);
            await new Promise((resolve, reject) => {
                socket.onopen = () => resolve(void 0);
                socket.onerror = () => reject();
                setTimeout(() => reject(), 500); // timeout
            });
            ws = socket;
            connectedPort = port;
            console.log("[extension] Connected to Tauri WebSocket on port", port);

            ws.onmessage = (event) => {
                // Handle messages from Tauri here
                console.log("[extension] Message from Tauri:", event.data);
            };
            break;
        } catch {
            // Try next port
        }
    }
    if (!ws) {
        console.warn("[extension] Could not connect to Tauri WebSocket server on any port");
    }
}

// Call this to send a message to Tauri
function sendMessageToTauri(obj) {
    if (ws && ws.readyState === WebSocket.OPEN) {
        ws.send(JSON.stringify(obj));
    } else {
        console.warn("[extension] WebSocket not connected");
    }
}

// Try to connect on load
connectToTauriWebSocket();

// --- Event sending helpers ---
function sendStepEvent(step) {
    if (ws && ws.readyState === WebSocket.OPEN) {
        ws.send(JSON.stringify(step));
    }
}

// --- Click Event ---
document.addEventListener('click', function(event) {
    const target = event.target;
    if (!(target instanceof HTMLElement)) return;
    const xpath = getXPath(target);
    const step = {
        type: "click",
        timestamp: Date.now(),
        tabId: 0, // You can update this if you have tab info
        url: window.location.href,
        xpath,
        cssSelector: getEnhancedCSSSelector(target, xpath),
        elementTag: target.tagName,
        elementText: target.textContent?.trim().slice(0, 200) || ""
    };
    sendStepEvent(step);
}, true);

// --- Input Event ---
document.addEventListener('input', function(event) {
    const target = event.target;
    if (!(target instanceof HTMLInputElement || target instanceof HTMLTextAreaElement)) return;
    const xpath = getXPath(target);
    const step = {
        type: "input",
        timestamp: Date.now(),
        tabId: 0,
        url: window.location.href,
        xpath,
        cssSelector: getEnhancedCSSSelector(target, xpath),
        elementTag: target.tagName,
        value: target.type === 'password' ? '********' : target.value
    };
    sendStepEvent(step);
}, true);

// --- KeyPress Event ---
const CAPTURED_KEYS = new Set([
    'Enter',
    'Tab',
    'Escape',
    'ArrowUp',
    'ArrowDown',
    'ArrowLeft',
    'ArrowRight',
    'Home',
    'End',
    'PageUp',
    'PageDown',
    'Backspace',
    'Delete',
]);
document.addEventListener('keydown', function(event) {
    const key = event.key;
    let keyToLog = '';
    // Capture explicit keys
    if (CAPTURED_KEYS.has(key)) {
        keyToLog = key;
    } else if (
        (event.ctrlKey || event.metaKey) &&
        key.length === 1 &&
        /[a-zA-Z0-9]/.test(key)
    ) {
        // Use 'CmdOrCtrl' to be cross-platform friendly in logs
        keyToLog = `CmdOrCtrl+${key.toUpperCase()}`;
    }
    if (!keyToLog) return;

    const target = event.target;
    let xpath = "";
    let cssSelector = "";
    let elementTag = "document";
    if (target && target.tagName) {
        xpath = getXPath(target);
        cssSelector = getEnhancedCSSSelector(target, xpath);
        elementTag = target.tagName;
    }
    const step = {
        type: "key_press",
        timestamp: Date.now(),
        tabId: 0,
        url: window.location.href,
        key: keyToLog,
        xpath,
        cssSelector,
        elementTag
    };
    sendStepEvent(step);
}, true);

// --- Scroll Event ---
let lastScrollTarget = null;
let lastScrollTimeout = null;
window.addEventListener('scroll', function(event) {
    if (lastScrollTimeout) clearTimeout(lastScrollTimeout);
    lastScrollTarget = event.target;
    lastScrollTimeout = setTimeout(() => {
        const target = lastScrollTarget || document.documentElement;
        const step = {
            type: "scroll",
            timestamp: Date.now(),
            tabId: 0,
            targetId: 0, // Not using rrweb, so just 0
            scrollX: window.scrollX,
            scrollY: window.scrollY
        };
        sendStepEvent(step);
    }, 300);
}, true);

// --- Navigation Event ---
(function() {
    let lastUrl = window.location.href;
    const pushState = history.pushState;
    const replaceState = history.replaceState;
    function sendNavStep(url) {
        const step = {
            type: "navigation",
            timestamp: Date.now(),
            tabId: 0,
            url
        };
        sendStepEvent(step);
    }
    history.pushState = function(...args) {
        pushState.apply(this, args);
        setTimeout(() => {
            if (window.location.href !== lastUrl) {
                lastUrl = window.location.href;
                sendNavStep(window.location.href);
            }
        }, 0);
    };
    history.replaceState = function(...args) {
        replaceState.apply(this, args);
        setTimeout(() => {
            if (window.location.href !== lastUrl) {
                lastUrl = window.location.href;
                sendNavStep(window.location.href);
            }
        }, 0);
    };
    window.addEventListener('popstate', function() {
        if (window.location.href !== lastUrl) {
            lastUrl = window.location.href;
            sendNavStep(window.location.href);
        }
    });
    // Initial page load
    sendNavStep(window.location.href);
})();
