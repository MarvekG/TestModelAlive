export function renderTestLogs(output: HTMLPreElement, chunks: string[]) {
  output.textContent = chunks.join("");
  output.scrollTop = output.scrollHeight;
}

export function appendTimestampedLog(chunks: string[], message: string) {
  const stamp = new Date().toLocaleTimeString("zh-CN", { hour12: false });
  chunks.push(`${stamp} ${message}\n`);
}
