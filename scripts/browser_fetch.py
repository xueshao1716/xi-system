#!/usr/bin/env python3
"""browser_fetch.py — 多策略浏览器渲染工具
策略1: r.jina.ai 代理 (最快, 返回Markdown)
策略2: iPhone MicroMessenger UA (适合微信文章)
策略3: 纯 requests 直连 (兜底)

用法: python3 browser_fetch.py <url> [timeout_seconds]
"""
import urllib.request, re, html, sys, json, os

def fetch_via_jina(url, timeout):
    """用 r.jina.ai 代理渲染"""
    jina_url = f"https://r.jina.ai/{url}"
    headers = {
        'User-Agent': 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36',
        'Accept': 'text/html,application/json,*/*',
        'X-Return-Format': 'text',
    }
    req = urllib.request.Request(jina_url, headers=headers)
    try:
        resp = urllib.request.urlopen(req, timeout=timeout)
        body = resp.read().decode('utf-8', 'replace')
        return body, 'jina'
    except Exception as e:
        return None, str(e)

def fetch_via_iphone_ua(url, timeout):
    """用 iPhone UA 抓微信文章"""
    headers = {
        'User-Agent': 'Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Mobile/15E148 MicroMessenger/8.0.50',
        'Accept': 'text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8',
    }
    req = urllib.request.Request(url, headers=headers)
    try:
        resp = urllib.request.urlopen(req, timeout=timeout)
        html_text = resp.read().decode('utf-8', 'replace')
        # Check if blocked
        if '环境异常' in html_text or '验证码' in html_text:
            return None, 'blocked'
        # Extract title
        title = ''
        for pat in [r'var msg_title = "(.*?)"', r"var msg_title = '(.*?)'", r'<meta property="og:title" content="(.*?)"']:
            m = re.search(pat, html_text, re.S)
            if m: title = html.unescape(m.group(1)); break
        # Extract body
        m = re.search(r'id="js_content"[^>]*>(.*?)</div>\s*<script', html_text, re.S)
        if m:
            body = m.group(1)
            body = re.sub(r'<script.*?</script>|<style.*?</style>', '', body, flags=re.S)
            body = re.sub(r'<br\s*/?>|</p>|</section>|</h\d>|</li>|</blockquote>', '\n', body, flags=re.I)
            body = re.sub(r'<[^>]+>', '', body)
            body = html.unescape(body)
            body = re.sub(r'[ \t\r\f\v]+', ' ', body)
            body = re.sub(r'\n\s*\n+', '\n', body).strip()
            result = f"标题: {title}\n来源: wechat\nURL: {url}\n\n{body}"
            return result, 'wechat'
        return None, 'no_js_content'
    except Exception as e:
        return None, str(e)

def fetch_via_direct(url, timeout):
    """纯 requests 直连"""
    headers = {
        'User-Agent': 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36',
        'Accept': 'text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8',
    }
    req = urllib.request.Request(url, headers=headers)
    try:
        resp = urllib.request.urlopen(req, timeout=timeout)
        body = resp.read().decode('utf-8', 'replace')
        # Strip HTML tags
        body = re.sub(r'<script.*?</script>|<style.*?</style>', '', body, flags=re.S)
        body = re.sub(r'<[^>]+>', '', body)
        body = re.sub(r'\s+', ' ', body).strip()
        return body[:20000], 'direct'
    except Exception as e:
        return None, str(e)

def main():
    url = sys.argv[1] if len(sys.argv) > 1 else ''
    timeout = int(sys.argv[2]) if len(sys.argv) > 2 else 30
    if not url:
        print("用法: browser_fetch.py <url> [timeout]")
        sys.exit(1)

    # Strategy 1: iPhone UA (WeChat 文章最稳定)
    result, source = fetch_via_iphone_ua(url, timeout)
    if result:
        print(f"SOURCE: {source}")
        print("---CONTENT---")
        print(result[:50000])
        return

    # Strategy 2: jina.ai (一般网页渲染)
    result, source = fetch_via_jina(url, timeout)
    if result:
        print(f"SOURCE: {source}")
        print("---CONTENT---")
        print(result[:50000])
        return

    # Strategy 3: direct (兜底)
    result, source = fetch_via_direct(url, timeout)
    if result:
        print(f"SOURCE: {source}")
        print("---CONTENT---")
        print(result[:50000])
        return

    print(f"ERROR: 所有策略失败 ({source})")
    sys.exit(2)

if __name__ == '__main__':
    main()
