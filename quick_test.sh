#!/bin/bash

# 简单的 fetch 功能验证
echo "=== 快速 Fetch 测试 ==="

cd /Users/macbook/Desktop/code/git/git

# 编译
echo "编译项目..."
cargo build

echo ""

# 创建简单的测试环境
TEST_DIR="/tmp/quick_fetch_test"
rm -rf "$TEST_DIR"
mkdir -p "$TEST_DIR/remote" "$TEST_DIR/local"

echo "创建测试仓库..."

# 1. 创建远程仓库（使用系统 git）
cd "$TEST_DIR/remote"
git init --bare

# 2. 创建工作仓库并推送内容
cd "$TEST_DIR"
git clone ./remote work
cd work
git config user.name "Test"
git config user.email "test@test.com"

echo "Test file content" > test.txt
git add test.txt
git commit -m "Test commit"
git push origin main

echo ""

# 3. 用我们的实现创建本地仓库
cd "$TEST_DIR/local"
/Users/macbook/Desktop/code/git/git/target/debug/git init

# 配置远程仓库
mkdir -p .git
echo "[remote \"origin\"]
	url = $TEST_DIR/remote
	fetch = +refs/heads/*:refs/remotes/origin/*" > .git/config

echo "配置完成，开始测试 fetch..."
echo ""

# 测试模拟模式
echo "=== 测试 1: 模拟模式 ==="
export GIT_FETCH_SIMULATE=1
/Users/macbook/Desktop/code/git/git/target/debug/git fetch origin 2>&1 || echo "模拟模式测试完成"

echo ""

# 测试本地模式
echo "=== 测试 2: 本地路径模式 ==="
unset GIT_FETCH_SIMULATE
/Users/macbook/Desktop/code/git/git/target/debug/git fetch origin 2>&1 || echo "本地模式测试完成"

echo ""
echo "=== 检查结果 ==="

echo "远程跟踪分支:"
find .git/refs/remotes -name "*" -type f 2>/dev/null | head -5

echo ""
echo "FETCH_HEAD:"
[ -f .git/FETCH_HEAD ] && head -3 .git/FETCH_HEAD || echo "FETCH_HEAD 不存在"

echo ""
echo "对象文件:"
find .git/objects -name "*" -type f 2>/dev/null | wc -l | xargs echo "对象数量:"

echo ""
echo "测试完成！"
