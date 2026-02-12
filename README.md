# IM-TT

IM-TT 是一个基于 Rust 的即时通讯应用，使用 Tauri 作为前端框架，Salvo + Tokio 作为后端服务器。

## 技术栈

### 后端
- **框架**: Salvo + Tokio
- **数据库**: MySQL (用于持久化存储)
- **缓存**: Redis (用于会话和缓存管理)
- **认证**: JWT (JSON Web Tokens)
- **密码加密**: bcrypt
- **实时通信**: WebSocket

### 前端
- **框架**: Tauri (Rust-based desktop application framework)
- **技术**: HTML5, CSS3, JavaScript (Vanilla)
- **UI设计**: 仿QQ界面风格

## 功能特性

- ✅ 用户注册和登录
- ✅ JWT 身份验证
- ✅ 个人聊天 (一对一)
- ✅ 群组聊天
- ✅ 实时消息推送 (WebSocket)
- ✅ 消息历史记录
- ✅ 在线状态显示
- ✅ 创建和加入群组

## 项目结构

```
IM-TT/
├── backend/              # 后端服务器
│   ├── src/
│   │   ├── models/      # 数据模型
│   │   ├── handlers/    # API 处理器
│   │   ├── db/          # 数据库连接
│   │   ├── utils/       # 工具函数 (JWT, 密码加密)
│   │   ├── websocket/   # WebSocket 处理
│   │   └── main.rs      # 主程序入口
│   ├── Cargo.toml       # Rust 依赖配置
│   └── .env.example     # 环境变量示例
│
└── frontend/            # 前端客户端
    ├── src/
    │   ├── index.html   # 主页面
    │   ├── styles.css   # 样式文件
    │   └── main.js      # 主逻辑
    ├── src-tauri/       # Tauri 配置
    └── package.json     # Node.js 依赖
```

## 环境要求

- Rust 1.70+
- Node.js 18+
- MySQL 8.0+
- Redis 6.0+

## 安装和运行

### 1. 数据库设置

#### 安装 MySQL
```bash
# Ubuntu/Debian
sudo apt-get install mysql-server

# macOS
brew install mysql

# 启动 MySQL
sudo systemctl start mysql  # Linux
brew services start mysql   # macOS
```

#### 创建数据库
```sql
mysql -u root -p
CREATE DATABASE im_tt CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;
```

#### 安装 Redis
```bash
# Ubuntu/Debian
sudo apt-get install redis-server

# macOS
brew install redis

# 启动 Redis
sudo systemctl start redis  # Linux
brew services start redis   # macOS
```

### 2. 后端设置

```bash
cd backend

# 复制环境变量文件
cp .env.example .env

# 编辑 .env 文件，配置数据库连接
# DATABASE_URL=mysql://root:your_password@localhost:3306/im_tt
# REDIS_URL=redis://localhost:6379
# JWT_SECRET=your_secret_key_here

# 构建并运行
cargo build --release
cargo run
```

后端服务器将在 `http://localhost:8080` 启动。

### 3. 前端设置

```bash
cd frontend

# 安装依赖
npm install

# 开发模式运行
npm run tauri dev

# 构建生产版本
npm run tauri build
```

## API 文档

### 认证 API

#### 注册
```http
POST /api/auth/register
Content-Type: application/json

{
  "username": "用户名",
  "email": "user@example.com",
  "password": "密码"
}
```

#### 登录
```http
POST /api/auth/login
Content-Type: application/json

{
  "email": "user@example.com",
  "password": "密码"
}
```

### 消息 API

所有消息 API 需要在 Header 中包含 JWT Token：
```
Authorization: Bearer <token>
```

#### 发送消息
```http
POST /api/messages/send
Content-Type: application/json

{
  "receiver_id": 2,        # 个人聊天时使用
  "group_id": null,        # 群聊时使用
  "content": "消息内容",
  "message_type": "text"
}
```

#### 获取消息历史
```http
GET /api/messages/list?receiver_id=2
# 或
GET /api/messages/list?group_id=1
```

### 群组 API

#### 创建群组
```http
POST /api/groups/create
Content-Type: application/json

{
  "name": "群组名称",
  "description": "群组描述"
}
```

#### 加入群组
```http
POST /api/groups/join
Content-Type: application/json

{
  "group_id": 1
}
```

#### 获取用户的群组列表
```http
GET /api/groups/list
```

### WebSocket

连接 WebSocket：
```javascript
ws://localhost:8080/api/ws
```

需要在 URL 参数或 Header 中传递 JWT Token。

WebSocket 消息格式：
```json
{
  "event_type": "message",
  "user_id": 1,
  "receiver_id": 2,
  "group_id": null,
  "content": "消息内容"
}
```

## 数据库架构

### users 表
```sql
- id: BIGINT (主键)
- username: VARCHAR(50) (唯一)
- email: VARCHAR(100) (唯一)
- password_hash: VARCHAR(255)
- avatar: VARCHAR(255)
- status: VARCHAR(20) (online/offline/away)
- created_at: TIMESTAMP
- updated_at: TIMESTAMP
```

### groups_table 表
```sql
- id: BIGINT (主键)
- name: VARCHAR(100)
- description: TEXT
- avatar: VARCHAR(255)
- owner_id: BIGINT (外键)
- created_at: TIMESTAMP
- updated_at: TIMESTAMP
```

### group_members 表
```sql
- id: BIGINT (主键)
- group_id: BIGINT (外键)
- user_id: BIGINT (外键)
- role: VARCHAR(20) (owner/admin/member)
- joined_at: TIMESTAMP
```

### messages 表
```sql
- id: BIGINT (主键)
- sender_id: BIGINT (外键)
- receiver_id: BIGINT (外键，可为空)
- group_id: BIGINT (外键，可为空)
- content: TEXT
- message_type: VARCHAR(20) (text/image/file)
- created_at: TIMESTAMP
- is_read: BOOLEAN
```

## 开发计划

- [ ] 添加文件上传功能
- [ ] 添加表情包支持
- [ ] 添加语音/视频通话
- [ ] 添加消息已读/未读状态
- [ ] 添加消息搜索功能
- [ ] 添加用户搜索和添加好友功能
- [ ] 优化 UI/UX
- [ ] 添加通知功能
- [ ] 添加群组管理功能 (踢人、禁言等)
- [ ] 添加端到端加密

## 许可证

MIT License

## 贡献

欢迎提交 Issue 和 Pull Request！
