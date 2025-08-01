#!/bin/bash

# fetch 功能测试脚本
set -e

echo "=== Git Fetch 功能测试 ==="

# 编译项目
echo "1. 编译项目..."
cargo build --release

# 创建测试目录
TEST_DIR="/tmp/git_fetch_test"
rm -rf "$TEST_DIR"
mkdir -p "$TEST_DIR"
cd "$TEST_DIR"

echo "2. 设置测试环境..."

# 创建一个测试用的远程仓库
REMOTE_REPO="$TEST_DIR/remote_repo"
mkdir -p "$REMOTE_REPO"
cd "$REMOTE_REPO"

# 初始化远程仓库
git init --bare
echo "远程仓库已创建: $REMOTE_REPO"

# 创建一个工作仓库来推送一些内容到远程仓库
WORK_REPO="$TEST_DIR/work_repo"
mkdir -p "$WORK_REPO"
cd "$WORK_REPO"

git init
git config user.name "Test User"
git config user.email "test@example.com"

# 添加远程仓库
git remote add origin "$REMOTE_REPO"

# 创建一些测试文件和提交
echo "Hello World" > README.md
git add README.md
git commit -m "Initial commit"

echo "Second line" >> README.md
git add README.md
git commit -m "Second commit"

# 推送到远程仓库
git push origin main

echo "工作仓库已设置完成"

# 创建测试用的本地仓库（用我们的实现）
LOCAL_REPO="$TEST_DIR/local_repo"
mkdir -p "$LOCAL_REPO"
cd "$LOCAL_REPO"

# 使用我们的 git 实现初始化
/Users/macbook/Desktop/code/git/git/target/release/git init

# 添加远程仓库配置
echo "[remote \"origin\"]
	url = $REMOTE_REPO
	fetch = +refs/heads/*:refs/remotes/origin/*" >> .git/config

echo "本地仓库已设置完成"
echo "远程仓库路径: $REMOTE_REPO"
echo "本地仓库路径: $LOCAL_REPO"

echo ""
echo "=== 开始测试 ==="
echo ""

# 测试 1: 模拟 fetch
echo "测试 1: 模拟 fetch (GIT_FETCH_SIMULATE=1)"
export GIT_FETCH_SIMULATE=1
/Users/macbook/Desktop/code/git/git/target/release/git fetch origin

echo ""

# 测试 2: 本地 fetch
echo "测试 2: 本地路径 fetch"
unset GIT_FETCH_SIMULATE
/Users/macbook/Desktop/code/git/git/target/release/git fetch origin

echo ""

# 检查结果
echo "=== 检查结果 ==="
echo "远程跟踪分支:"
find .git/refs/remotes -type f -exec echo {} \; -exec cat {} \; 2>/dev/null || echo "没有找到远程跟踪分支"

echo ""
echo "FETCH_HEAD 内容:"
cat .git/FETCH_HEAD 2>/dev/null || echo "FETCH_HEAD 不存在"

echo ""
echo "对象数量:"
find .git/objects -type f | wc -l

echo ""
echo "测试完成！"
