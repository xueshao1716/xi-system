"""
微信公众号文章阅读器
支持单篇抓取和批量抓取
"""
import json
import sys
import io
import re
import urllib.request
import urllib.error
from pathlib import Path
from datetime import datetime

WORKSPACE = Path(__file__).parent.parent
CACHE_DIR = WORKSPACE / "memory" / "wechat-articles"


def fetch_article(url: str) -> dict:
    """抓取一篇微信公众号文章"""
    if "mp.weixin.qq.com" not in url:
        return {"error": "不是微信公众号文章链接"}

    headers = {
        "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36",
        "Accept": "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
        "Accept-Language": "zh-CN,zh;q=0.9,en;q=0.8",
    }

    req = urllib.request.Request(url, headers=headers)

    try:
        with urllib.request.urlopen(req, timeout=30) as resp:
            html = resp.read().decode("utf-8", errors="replace")
    except urllib.error.HTTPError as e:
        return {"error": f"HTTP {e.code}: {e.reason}"}
    except Exception as e:
        return {"error": str(e)}

    # 提取标题
    title_match = re.search(r'<h1[^>]*id="activity-name"[^>]*>(.*?)</h1>', html, re.DOTALL)
    if not title_match:
        title_match = re.search(r'<h1[^>]*>(.*?)</h1>', html, re.DOTALL)
    title = title_match.group(1).strip() if title_match else "未知标题"
    title = re.sub(r'<[^>]+>', '', title).strip()

    # 提取作者
    author_match = re.search(r'id="js_name"[^>]*>(.*?)</a>', html, re.DOTALL)
    if not author_match:
        author_match = re.search(r'class="rich_media_meta[^"]*"[^>]*>(.*?)</span>', html, re.DOTALL)
    author = author_match.group(1).strip() if author_match else "未知作者"
    author = re.sub(r'<[^>]+>', '', author).strip()

    # 提取正文
    content_match = re.search(r'id="js_content"[^>]*>(.*?)</div>\s*<script', html, re.DOTALL)
    if not content_match:
        content_match = re.search(r'id="js_content"[^>]*>(.*)', html, re.DOTALL)

    if content_match:
        raw_content = content_match.group(1)
        # 清理 HTML 标签
        content = re.sub(r'<br\s*/?>', '\n', raw_content)
        content = re.sub(r'<p[^>]*>', '\n', content)
        content = re.sub(r'</p>', '', content)
        content = re.sub(r'<img[^>]*data-src="([^"]*)"[^>]*>', '[图片: \\1]', content)
        content = re.sub(r'<[^>]+>', '', content)
        content = re.sub(r'&nbsp;', ' ', content)
        content = re.sub(r'&lt;', '<', content)
        content = re.sub(r'&gt;', '>', content)
        content = re.sub(r'&amp;', '&', content)
        content = re.sub(r'\n{3,}', '\n\n', content)
        content = content.strip()
    else:
        content = "无法提取正文（可能需要在微信客户端内打开）"

    # 提取发布时间
    date_match = re.search(r'var\s+ct\s*=\s*"(\d+)"', html)
    publish_time = ""
    if date_match:
        try:
            ts = int(date_match.group(1))
            publish_time = datetime.fromtimestamp(ts).strftime("%Y-%m-%d %H:%M")
        except:
            pass

    return {
        "title": title,
        "author": author,
        "publish_time": publish_time,
        "url": url,
        "content": content,
        "content_length": len(content),
    }


def save_article(article: dict) -> Path:
    """保存文章到本地"""
    CACHE_DIR.mkdir(parents=True, exist_ok=True)
    date_str = datetime.now().strftime("%Y-%m-%d")
    safe_title = re.sub(r'[\\/:*?"<>|]', '_', article.get("title", "untitled"))[:50]
    filename = f"{date_str}--{safe_title}.md"
    path = CACHE_DIR / filename

    md = f"""# {article['title']}

- 作者：{article['author']}
- 发布时间：{article.get('publish_time', '未知')}
- 来源：{article['url']}
- 字数：{article['content_length']}

---

{article['content']}
"""
    path.write_text(md, encoding="utf-8")
    return path


def read_wechat(url: str, save: bool = True) -> str:
    """一站式读取微信文章"""
    article = fetch_article(url)

    if "error" in article:
        return f"抓取失败: {article['error']}"

    if save:
        path = save_article(article)
        save_note = f"\n已保存: {path.name}"
    else:
        save_note = ""

    return f"""【{article['title']}】
作者: {article['author']}  |  {article.get('publish_time', '')}
字数: {article['content_length']}

{article['content'][:2000]}{'...' if article['content_length'] > 2000 else ''}{save_note}"""


if __name__ == "__main__":
    sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding="utf-8")

    if len(sys.argv) < 2:
        print("用法:")
        print("  python wechat_reader.py <文章URL>")
        print("  python wechat_reader.py batch <URL1> <URL2> ...")
        print("  python wechat_reader.py --list  # 列出已缓存文章")
        sys.exit(1)

    if sys.argv[1] == "--list":
        if CACHE_DIR.exists():
            for f in sorted(CACHE_DIR.glob("*.md")):
                print(f"  {f.name}")
        else:
            print("暂无缓存文章")
        sys.exit(0)

    if sys.argv[1] == "batch":
        urls = sys.argv[2:]
        for url in urls:
            print(read_wechat(url))
            print()
    else:
        print(read_wechat(sys.argv[1]))
