#!/bin/bash

echo "=== 简单 Fetch 测试 ==="

# 创建测试目录
TEST_DIR="/tmp/simple_fetch_test"
rm -rf "$TEST_DIR"
mkdir -p "$TEST_DIR/remote" "$TEST_DIR/local"

echo "1. 创建测试环境..."

# 创建远程仓库
cd "$TEST_DIR/remote"
git init --bare

# 创建工作仓库
cd "$TEST_DIR"
git clone ./remote work
cd work
git config user.name "Test"
git config user.email "test@test.com"

echo "Hello Fetch!" > test.txt
git add test.txt
git commit -m "Initial commit"
git push origin main

echo "2. 设置本地仓库..."

# 用我们的实现创建本地仓库
cd "$TEST_DIR/local"
/Users/macbook/Desktop/code/git/git/target/debug/git init

# 手动创建配置
echo "[remote \"origin\"]
	url = $TEST_DIR/remote
	fetch = +refs/heads/*:refs/remotes/origin/*" > .git/config

echo "3. 测试 fetch..."

echo "--- 模拟模式 ---"
export GIT_FETCH_SIMULATE=1
/Users/macbook/Desktop/code/git/git/target/debug/git fetch origin

echo ""
echo "--- 本地模式 ---"
unset GIT_FETCH_SIMULATE
/Users/macbook/Desktop/code/git/git/target/debug/git fetch origin

echo ""
echo "4. 检查结果..."
echo "远程跟踪分支:"
find .git/refs/remotes 2>/dev/null || echo "无远程跟踪分支"

echo ""
echo "FETCH_HEAD:"
cat .git/FETCH_HEAD 2>/dev/null || echo "无 FETCH_HEAD"

echo ""
echo "测试完成！"
