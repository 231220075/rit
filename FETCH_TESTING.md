# Git Fetch 功能测试指南

## 快速测试

运行快速测试脚本：
```bash
./quick_test.sh
```

## 手动测试步骤

### 1. 编译项目
```bash
cargo build
```

### 2. 创建测试环境

#### 创建测试目录
```bash
mkdir -p /tmp/fetch_test/{remote,local}
cd /tmp/fetch_test
```

#### 创建远程仓库
```bash
cd remote
git init --bare
```

#### 创建工作仓库并推送内容
```bash
cd ..
git clone ./remote work
cd work
git config user.name "Test User"
git config user.email "test@example.com"

echo "Hello Fetch Test" > README.md
git add README.md
git commit -m "Initial commit"

echo "Additional content" >> README.md
git add README.md
git commit -m "Second commit"

git push origin main
cd ..
```

#### 用我们的实现创建本地仓库
```bash
cd local
/path/to/your/git/target/debug/git init

# 手动添加远程配置
echo '[remote "origin"]
    url = /tmp/fetch_test/remote
    fetch = +refs/heads/*:refs/remotes/origin/*' >> .git/config
```

### 3. 测试不同的 fetch 模式

#### 测试 1: 模拟模式（用于调试）
```bash
export GIT_FETCH_SIMULATE=1
/path/to/your/git/target/debug/git fetch origin
```

#### 测试 2: 本地路径模式
```bash
unset GIT_FETCH_SIMULATE
/path/to/your/git/target/debug/git fetch origin
```

#### 测试 3: 详细模式
```bash
/path/to/your/git/target/debug/git fetch -v origin
```

### 4. 验证结果

#### 检查远程跟踪分支
```bash
find .git/refs/remotes -type f -exec echo "=== {} ===" \; -exec cat {} \;
```

#### 检查 FETCH_HEAD
```bash
cat .git/FETCH_HEAD
```

#### 检查下载的对象
```bash
find .git/objects -type f | wc -l
ls -la .git/objects/*/
```

#### 验证对象内容（如果有的话）
```bash
# 查看 commit 对象
find .git/objects -type f | head -1 | xargs /path/to/your/git/target/debug/git cat-file -p
```

## HTTP 测试（如果有公开仓库）

### 创建 HTTP 远程配置
```bash
echo '[remote "github"]
    url = https://github.com/user/repo.git
    fetch = +refs/heads/*:refs/remotes/github/*' >> .git/config
```

### 测试 HTTP fetch
```bash
/path/to/your/git/target/debug/git fetch github
```

## 期望的结果

### 成功的标志：
1. **远程跟踪分支创建**: `.git/refs/remotes/origin/main` 文件存在且包含正确的提交哈希
2. **FETCH_HEAD 文件**: 包含获取的引用信息
3. **对象下载**: `.git/objects/` 目录中有新的对象文件
4. **无错误输出**: 程序正常执行没有崩溃

### 输出示例：
```
Fetching from origin...
 * [new branch]      main -> origin/main
Fetched 1 reference(s)
```

## 故障排除

### 常见问题：
1. **"Remote 'origin' not found"**: 检查 `.git/config` 文件中的远程配置
2. **"Remote path does not exist"**: 确保远程仓库路径正确
3. **网络错误**: 检查 HTTP URL 是否可访问
4. **协议错误**: 查看详细的错误信息，可能是协议解析问题

### 调试技巧：
1. 使用 `-v` 参数获取详细输出
2. 先测试模拟模式确保基本逻辑正确
3. 使用本地路径测试避免网络问题
4. 检查 `.git/objects` 目录确认对象是否被创建

## 性能测试

### 大仓库测试：
创建包含多个提交和文件的仓库进行测试：
```bash
# 在工作仓库中
for i in {1..10}; do
    echo "Content $i" > "file$i.txt"
    git add "file$i.txt"
    git commit -m "Commit $i"
done
git push origin main
```

然后测试 fetch 性能和正确性。
