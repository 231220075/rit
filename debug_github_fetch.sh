#!/bin/bash

echo "=== GitHub Fetch 调试测试 ==="

# 清理之前的测试
rm -rf /tmp/github_fetch_debug
mkdir -p /tmp/github_fetch_debug
cd /tmp/github_fetch_debug

echo "1. 初始化仓库..."
/Users/macbook/Desktop/code/git/git/target/debug/git init

echo "2. 配置远程仓库..."
echo '[remote "origin"]
    url = https://github.com/231220075/rit.git
    fetch = +refs/heads/*:refs/remotes/origin/*' > .git/config

echo "3. 先测试引用发现..."
echo "使用 curl 测试 GitHub 的 info/refs 端点："
curl -s "https://github.com/231220075/rit.git/info/refs?service=git-upload-pack" | head -10

echo ""
echo "4. 运行详细的 fetch..."
/Users/macbook/Desktop/code/git/git/target/debug/git fetch -v origin

echo ""
echo "5. 检查结果..."
echo "远程跟踪分支:"
find .git/refs/remotes 2>/dev/null || echo "没有远程跟踪分支"

echo ""
echo "FETCH_HEAD:"
cat .git/FETCH_HEAD 2>/dev/null || echo "没有 FETCH_HEAD"

echo ""
echo "对象文件:"
find .git/objects -name "??" -type d 2>/dev/null | wc -l | xargs echo "对象目录数量:"

echo ""
echo "调试完成！"
