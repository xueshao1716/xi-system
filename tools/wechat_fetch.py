#!/usr/bin/env python3
"""
微信公众号文章抓取工具 v3
基于 wechat-article-fetch skill 的完整实现。

三种模式：
  1. 无 Cookie 模式（默认）— iPhone MicroMessenger UA
  2. 有 Cookie 模式 — 带登录态
  3. 短内容/流式文章模式 — og:description 兜底

用法：
  python3 wechat_fetch.py <URL>              抓取文章
  python3 wechat_fetch.py --set-cookie "str"  保存Cookie
  python3 wechat_fetch.py --check             检查Cookie
  python3 wechat_fetch.py --json <URL>        JSON输出（给程序调用）
"""

import urllib.request
import re
import html as html_mod
import json
import os
import sys
from datetime import datetime

COOKIE_FILE = os.path.join(os.path.dirname(os.path.abspath(__file__)), ".wechat_cookies.json")

UA_MOBILE = "Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Mobile/15E148 MicroMessenger/8.0.50(0x18003230) NetType/WIFI Language/zh_CN"
UA_DESKTOP = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/134.0.0.0 Safari/537.36 MicroMessenger/7.0.20.1781 NetType/WIFI"


def _js_x_decode(s):
    """解码 JS \\xNN 转义 + HTML entities"""
    if s is None:
        return None
    s = re.sub(r'\\x([0-9a-fA-F]{2})', lambda m: chr(int(m.group(1), 16)), s)
    return html_mod.unescape(s).replace('\\/', '/').strip()


def _load_cookies():
    if not os.path.exists(COOKIE_FILE):
        return {}
    try:
        with open(COOKIE_FILE) as f:
            return json.load(f)
    except Exception:
        return {}


def _save_cookies(cookie_str):
    cookies = {}
    for item in cookie_str.split(";"):
        item = item.strip()
        if "=" in item:
            k, v = item.split("=", 1)
            cookies[k.strip()] = v.strip()
    if cookies:
        with open(COOKIE_FILE, "w") as f:
            json.dump(cookies, f, indent=2)
    return cookies


def _fetch(url, cookies=None):
    """统一请求入口，绕过代理"""
    headers = {
        "User-Agent": UA_MOBILE if not cookies else UA_DESKTOP,
        "Accept": "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
        "Accept-Language": "zh-CN,zh;q=0.9,en;q=0.8",
    }
    if cookies:
        headers["Cookie"] = "; ".join(f"{k}={v}" for k, v in cookies.items())
        headers["Referer"] = "https://mp.weixin.qq.com/"

    opener = urllib.request.build_opener(urllib.request.ProxyHandler({}))
    req = urllib.request.Request(url, headers=headers)
    resp = opener.open(req, timeout=30)
    return resp.read().decode("utf-8", "replace")


def _wechat_var(text, name):
    """从页面源码提取微信全局JS变量，支持多种格式"""
    patterns = [
        # var msg_title = '标题'.html(false);
        rf'var\s+{re.escape(name)}\s*=\s*(["\'])([\s\S]*?)\1\.html\(false\)\s*;',
        # var msg_desc = htmlDecode("...");
        rf'var\s+{re.escape(name)}\s*=\s*htmlDecode\((["\'])([\s\S]*?)\1\)\s*;',
        # window.msg_title = '...'
        rf'window\.{re.escape(name)}\s*=\s*(["\'])([^"\']*)\1\s*;',
        # var msg_title = '...' (普通赋值)
        rf'var\s+{re.escape(name)}\s*=\s*(["\'])([^"\']*)\1\s*;',
    ]
    for p in patterns:
        m = re.search(p, text)
        if m:
            return _js_x_decode(m.group(2))
    return None


def _meta_prop(text, prop):
    """提取 meta 标签的 property/content"""
    m = re.search(
        rf'<meta[^>]+(?:property|name)=["\']' + re.escape(prop) +
        rf'["\'][^>]+content=["\']([\s\S]*?)["\'][^>]*>', text, re.I)
    if not m:
        m = re.search(
            rf'<meta[^>]+content=["\']([\s\S]*?)["\'][^>]+(?:property|name)=["\']' +
            re.escape(prop) + rf'["\'][^>]*>', text, re.I)
    return _js_x_decode(m.group(1)) if m else None


def _extract(html_text):
    """从HTML提取文章内容，支持多种页面格式"""
    # === 检测拦截页 ===
    if "环境异常" in html_text or "验证码" in html_text:
        return {"error": "BLOCKED", "detail": "微信验证页拦截，需要Cookie或换网络"}

    # === 提取元数据 ===
    title = _wechat_var(html_text, 'msg_title') or _meta_prop(html_text, 'og:title') or ""
    author = _wechat_var(html_text, 'nickname') or ""
    desc = _wechat_var(html_text, 'msg_desc') or _meta_prop(html_text, 'og:description') or ""
    cover = _wechat_var(html_text, 'msg_cdn_url') or _meta_prop(html_text, 'og:image') or ""
    publish_time = _wechat_var(html_text, 'ct') or ""

    # === 提取正文（按顺序尝试多种模式）===
    content = ""

    # 模式1：标准 js_content 到 script
    m = re.search(r'id="js_content"[^>]*>(.*?)</div>\s*<script', html_text, re.DOTALL)
    if not m:
        # 模式2：带 visibility 样式的 js_content
        m = re.search(r'id="js_content"[^>]*style="[^"]*visibility:\s*(?:visible|hidden)[^"]*"[^>]*>(.*?)</div>\s*<script', html_text, re.DOTALL)
    if not m:
        # 模式3：rich_media_content 兜底
        m = re.search(r'class="rich_media_content[^"]*"[^>]*id="js_content"[^>]*>(.*?)</div>\s*(?:<\w+|$)', html_text, re.DOTALL)

    if m:
        raw = m.group(1)
        # 清理 HTML
        raw = re.sub(r'<script.*?</script>|<style.*?</style>', '', raw, flags=re.DOTALL)
        raw = re.sub(r'<br\s*/?>|</p>|</section>|</h\d>|</li>|</blockquote>', '\n', raw, flags=re.I)
        raw = re.sub(r'<[^>]+>', '', raw)
        content = html_mod.unescape(raw)
        content = re.sub(r'[ \t\r\f\v]+', ' ', content)
        content = re.sub(r'\n\s*\n+', '\n', content).strip()

    # 模式4：短内容/流式文章 — 用 og:description 兜底
    if not content and desc and len(desc) > 100:
        content = desc
        content = content.replace('\\n', '\n').replace('\\x0a', '\n')
        content = html_mod.unescape(content)

    # 模式5：cgiDataNew 格式
    if not content:
        m = re.search(r'window\.cgiDataNew\s*=\s*\{[\s\S]*?\}', html_text)
        if m:
            # 尝试从 cgiDataNew 提取
            block = m.group(0)
            title_m = re.search(r"title:\s*JsDecode\('([\s\S]*?)'\)", block)
            desc_m = re.search(r"desc:\s*JsDecode\('([\s\S]*?)'\)", block)
            if desc_m:
                content = _js_x_decode(desc_m.group(1))
            if title_m and not title:
                title = _js_x_decode(title_m.group(1))

    if not content:
        return {"error": "NO_CONTENT", "detail": "正文提取失败，可能需要Cookie"}

    return {
        "title": title or "(未提取到标题)",
        "author": author or "(未提取到作者)",
        "publish_time": publish_time,
        "cover": cover,
        "content": content,
        "word_count": len(content),
    }


def fetch_article(url):
    """
    抓取微信公众号文章。
    优先无Cookie模式，失败再试Cookie模式。
    """
    if not url.startswith("http"):
        url = "https://mp.weixin.qq.com/s/" + url

    # 1. 无Cookie模式
    try:
        html_text = _fetch(url)
        result = _extract(html_text)
        if result.get("content") and not result.get("error"):
            result["mode"] = "no_cookie"
            return result
        no_cookie_error = result.get("error", "NO_CONTENT")
    except Exception as e:
        no_cookie_error = str(e)

    # 2. Cookie模式
    cookies = _load_cookies()
    if cookies:
        try:
            html_text = _fetch(url, cookies)
            result = _extract(html_text)
            if result.get("content") and not result.get("error"):
                result["mode"] = "with_cookie"
                return result
        except Exception:
            pass

    return {
        "error": "FETCH_FAILED",
        "detail": f"无Cookie: {no_cookie_error}。Cookie: {'无Cookie文件' if not cookies else '也失败'}",
        "title": "",
        "content": "",
    }


def main():
    if len(sys.argv) < 2:
        print("""微信公众号文章抓取工具 v3

用法:
  python3 wechat_fetch.py <URL>              抓取文章
  python3 wechat_fetch.py --json <URL>       JSON输出
  python3 wechat_fetch.py --set-cookie <s>   保存Cookie
  python3 wechat_fetch.py --check            检查Cookie
""")
        sys.exit(0)

    cmd = sys.argv[1]

    if cmd in ("--set-cookie", "-c"):
        if len(sys.argv) < 3:
            print("[ERROR] 请提供Cookie字符串", file=sys.stderr)
            sys.exit(1)
        cookies = _save_cookies(sys.argv[2])
        print(f"[OK] Cookie已保存 ({len(cookies)}条)")

    elif cmd == "--check":
        cookies = _load_cookies()
        if cookies:
            print(f"[OK] 有 {len(cookies)} 条Cookie")
        else:
            print("[INFO] 无Cookie文件，将使用无Cookie模式")

    elif cmd == "--json":
        if len(sys.argv) < 3:
            print("[ERROR] 请提供URL", file=sys.stderr)
            sys.exit(1)
        result = fetch_article(sys.argv[2])
        print(json.dumps(result, ensure_ascii=False, indent=2))
        if result.get("error"):
            sys.exit(1)

    else:
        url = cmd
        result = fetch_article(url)

        if result.get("error"):
            print(f"[ERROR] {result['error']}: {result.get('detail', '')}", file=sys.stderr)
            sys.exit(1)

        mode = result.get("mode", "unknown")
        print(f"\n{'='*60}")
        print(f"📰 {result['title']}")
        print(f"✍️  {result['author']}")
        if result.get("publish_time"):
            try:
                ts = int(result["publish_time"])
                dt = datetime.fromtimestamp(ts)
                print(f"📅 {dt.strftime('%Y-%m-%d %H:%M')}")
            except Exception:
                pass
        print(f"📝 {result['word_count']}字 | 模式: {mode}")
        print(f"{'='*60}\n")
        print(result["content"])
        print(f"\n{'='*60}")


if __name__ == "__main__":
    main()
