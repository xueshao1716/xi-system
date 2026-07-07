#!/usr/bin/env bash
# browser_fetch.sh — 用 npx playwright CLI 渲染页面
# 用法: browser_fetch.sh <url> [timeout_seconds]
# 依赖: npx, playwright (自动安装)

set -euo pipefail

URL="${1:?用法: browser_fetch.sh <url> [timeout_seconds]}"
TIMEOUT="${2:-30}"

# 用 Playwright 的 eval 直接在浏览器里执行 JS 提取内容
# 不依赖 require('playwright')，直接用 npx playwright CLI 的 evaluate 模式
SCRIPT=$(cat << 'JSEOF'
const url = process.argv[1];
const timeout = parseInt(process.argv[2] || '30') * 1000;

(async () => {
    const { chromium } = await import('playwright');
    const browser = await chromium.launch({
        headless: true,
        args: ['--no-sandbox', '--disable-setuid-sandbox', '--disable-dev-shm-usage', '--disable-gpu', '--single-process'],
    });
    const page = await browser.newPage();
    await page.goto(url, { waitUntil: 'domcontentloaded', timeout }).catch(() => {});
    await page.waitForTimeout(3000);

    const title = await page.title().catch(() => '');
    console.log(`TITLE: ${title}`);
    console.log(`URL: ${url}`);

    // Try multiple content extraction strategies
    let content = '';

    // Strategy 1: WeChat js_content
    content = await page.evaluate(() => {
        const el = document.getElementById('js_content');
        return el ? el.innerText : '';
    }).catch(() => '');
    if (content && content.length > 100) { console.log('SOURCE: js_content'); printContent(content); await browser.close(); return; }

    // Strategy 2: article tag
    content = await page.evaluate(() => {
        const el = document.querySelector('article');
        return el ? el.innerText : '';
    }).catch(() => '');
    if (content && content.length > 200) { console.log('SOURCE: article'); printContent(content); await browser.close(); return; }

    // Strategy 3: body
    content = await page.evaluate(() => {
        const body = document.body;
        if (!body) return '';
        const text = body.innerText || '';
        const lines = text.split('\n').filter((l, i, arr) => { const t = l.trim(); return t && arr.indexOf(l) === i; });
        return lines.join('\n');
    }).catch(() => '');
    console.log('SOURCE: body');
    printContent(content || '(empty)');

    await browser.close();
})().catch(e => { console.error(`ERROR: ${e.message}`); process.exit(1); });

function printContent(text) {
    console.log('---CONTENT---');
    console.log(text.trim().substring(0, 50000));
}
JSEOF
)

# Use node with --experimental-modules for dynamic import
node --input-type=module -e "$SCRIPT" "$URL" "$TIMEOUT" 2>&1