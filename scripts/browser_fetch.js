/**
 * 浏览器渲染工具 — 用 Playwright 渲染 JS 页面并返回纯文本
 * 用法: node browser_fetch.js <url> [timeout_seconds]
 * 输出: 页面标题 + 正文纯文本（sections 分隔）
 * 依赖: npx playwright (自动安装 chromium)
 */

const { chromium } = require('playwright');

async function main() {
    const url = process.argv[2];
    if (!url) {
        console.error('[browser] 用法: node browser_fetch.js <url> [timeout_seconds]');
        process.exit(1);
    }

    const timeout = (parseInt(process.argv[3]) || 30) * 1000;

    let browser;
    try {
        browser = await chromium.launch({
            headless: true,
            args: [
                '--no-sandbox',
                '--disable-setuid-sandbox',
                '--disable-dev-shm-usage',
                '--disable-gpu',
                '--single-process',
            ],
        });

        const context = await browser.newContext({
            userAgent: 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36',
            viewport: { width: 1280, height: 720 },
        });

        const page = await context.newPage();

        // 导航到页面
        await page.goto(url, {
            waitUntil: 'networkidle',
            timeout: timeout,
        }).catch(e => {
            // networkidle 超时但页面可能已经加载了内容，继续
            console.error('[browser] 导航警告:', e.message.substring(0, 100));
        });

        // 等待额外时间让 JS 渲染
        await page.waitForTimeout(2000);

        // 提取标题
        const title = await page.title().catch(() => '');
        console.log(`TITLE: ${title}`);
        console.log(`URL: ${url}`);

        // 提取正文 — 尝试多种策略
        let content = '';

        // 策略1: 微信公众号 js_content
        content = await page.evaluate(() => {
            const el = document.getElementById('js_content');
            return el ? el.innerText : '';
        }).catch(() => '');

        if (content && content.length > 100) {
            console.log('SOURCE: js_content');
            console.log('---CONTENT---');
            console.log(content.trim().substring(0, 50000));
            return;
        }

        // 策略2: article 标签
        content = await page.evaluate(() => {
            const el = document.querySelector('article');
            return el ? el.innerText : '';
        }).catch(() => '');

        if (content && content.length > 200) {
            console.log('SOURCE: article');
            console.log('---CONTENT---');
            console.log(content.trim().substring(0, 50000));
            return;
        }

        // 策略3: main content
        content = await page.evaluate(() => {
            const el = document.querySelector('main') || document.querySelector('.post-content') || document.querySelector('.entry-content');
            return el ? el.innerText : '';
        }).catch(() => '');

        if (content && content.length > 200) {
            console.log('SOURCE: main');
            console.log('---CONTENT---');
            console.log(content.trim().substring(0, 50000));
            return;
        }

        // 策略4: 获取 body 所有文本（去重）
        content = await page.evaluate(() => {
            const body = document.body;
            if (!body) return '';
            const text = body.innerText || '';
            // 去重：移除重复行
            const lines = text.split('\n').filter((l, i, arr) => {
                const trimmed = l.trim();
                return trimmed && arr.indexOf(l) === i;
            });
            return lines.join('\n');
        }).catch(() => '');

        console.log('SOURCE: body');
        console.log('---CONTENT---');
        console.log((content || '(空页面)').trim().substring(0, 50000));

    } catch (err) {
        console.error(`[browser] 错误: ${err.message.substring(0, 200)}`);
        process.exit(2);
    } finally {
        if (browser) await browser.close().catch(() => {});
    }
}

main().catch(err => {
    console.error('[browser] 致命错误:', err.message.substring(0, 200));
    process.exit(3);
});
