/**
 * @module live_cdp
 * @description DevTools helper for tests/live/webviews.ps1: lists the page URLs of a WebView2 browser, or
 *              evaluates an expression in the first page whose URL contains a substring.
 *
 * @input  `pages <port>` or `eval <port> <url-part> <expression>`.
 * @output One line: comma-separated page file names, or the JSON value of the expression ("" if none).
 * @dependencies none (Node 22 fetch + WebSocket)
 */
const [mode, port, match, expression] = process.argv.slice(2);
const targets = await fetch(`http://127.0.0.1:${port}/json`).then((r) => r.json()).catch(() => []);
const pages = targets.filter((t) => t.type === 'page');
if (mode === 'pages') {
  console.log(pages.map((p) => p.url.split('/').pop()).join(', '));
} else {
  const page = pages.find((p) => p.url.includes(match));
  if (!page) {
    console.log('');
  } else {
    const ws = new WebSocket(page.webSocketDebuggerUrl);
    const value = await new Promise((resolve) => {
      const timer = setTimeout(() => resolve(''), 5000);
      ws.onopen = () => ws.send(JSON.stringify({ id: 1, method: 'Runtime.evaluate', params: { expression, returnByValue: true } }));
      ws.onmessage = (m) => { clearTimeout(timer); resolve(JSON.stringify(JSON.parse(m.data).result?.result?.value ?? '')); };
      ws.onerror = () => { clearTimeout(timer); resolve(''); };
    });
    console.log(value);
    ws.close();
  }
}
