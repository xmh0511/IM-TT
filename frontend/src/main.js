// API Configuration
const API_BASE_URL = 'http://localhost:8080/api';
const WS_URL = 'ws://localhost:8080/api/ws';

// State Management
let currentUser = null;
let authToken = null;
let wsConnection = null;
let currentChat = null; // { type: 'contact'|'group', id: number, name: string }
let contacts = [];
let groups = [];
let messages = {};

// Page Navigation
function showPage(pageId) {
  document.querySelectorAll('.page').forEach(page => page.classList.add('hidden'));
  document.getElementById(pageId).classList.remove('hidden');
}

// API Calls
async function apiCall(endpoint, options = {}) {
  const headers = {
    'Content-Type': 'application/json',
    ...(authToken && { 'Authorization': `Bearer ${authToken}` }),
    ...options.headers
  };

  const response = await fetch(`${API_BASE_URL}${endpoint}`, {
    ...options,
    headers
  });

  if (!response.ok) {
    const error = await response.json().catch(() => ({ error: 'Request failed' }));
    throw new Error(error.error || 'Request failed');
  }

  return response.json();
}

// Authentication
async function register(username, email, password) {
  const data = await apiCall('/auth/register', {
    method: 'POST',
    body: JSON.stringify({ username, email, password })
  });
  
  authToken = data.token;
  currentUser = data.user;
  localStorage.setItem('authToken', authToken);
  localStorage.setItem('currentUser', JSON.stringify(currentUser));
  
  return data;
}

async function login(email, password) {
  const data = await apiCall('/auth/login', {
    method: 'POST',
    body: JSON.stringify({ email, password })
  });
  
  authToken = data.token;
  currentUser = data.user;
  localStorage.setItem('authToken', authToken);
  localStorage.setItem('currentUser', JSON.stringify(currentUser));
  
  return data;
}

function logout() {
  authToken = null;
  currentUser = null;
  localStorage.removeItem('authToken');
  localStorage.removeItem('currentUser');
  if (wsConnection) {
    wsConnection.close();
    wsConnection = null;
  }
  showPage('login-page');
}

// WebSocket Connection
function connectWebSocket() {
  if (!authToken) return;

  wsConnection = new WebSocket(`${WS_URL}?token=${authToken}`);
  
  wsConnection.onopen = () => {
    console.log('WebSocket connected');
  };

  wsConnection.onmessage = (event) => {
    try {
      const data = JSON.parse(event.data);
      handleWebSocketMessage(data);
    } catch (error) {
      console.error('Error parsing WebSocket message:', error);
    }
  };

  wsConnection.onclose = () => {
    console.log('WebSocket disconnected');
    // Reconnect after 3 seconds
    setTimeout(connectWebSocket, 3000);
  };

  wsConnection.onerror = (error) => {
    console.error('WebSocket error:', error);
  };
}

function handleWebSocketMessage(data) {
  if (data.event_type === 'message') {
    // Add message to messages
    const chatKey = data.group_id ? `group_${data.group_id}` : `contact_${data.user_id}`;
    if (!messages[chatKey]) {
      messages[chatKey] = [];
    }
    
    const message = {
      id: Date.now(),
      sender_id: data.user_id,
      receiver_id: data.receiver_id,
      group_id: data.group_id,
      content: data.content,
      message_type: 'text',
      created_at: new Date().toISOString(),
      is_read: false
    };
    
    messages[chatKey].push(message);
    
    // Update UI if this is the current chat
    if (currentChat) {
      const currentChatKey = currentChat.type === 'group' 
        ? `group_${currentChat.id}` 
        : `contact_${currentChat.id}`;
      
      if (chatKey === currentChatKey) {
        renderMessages();
      }
    }
  }
}

function sendWebSocketMessage(data) {
  if (wsConnection && wsConnection.readyState === WebSocket.OPEN) {
    wsConnection.send(JSON.stringify(data));
  }
}

// Messaging
async function sendMessage(content, receiverId = null, groupId = null) {
  const data = await apiCall('/messages/send', {
    method: 'POST',
    body: JSON.stringify({
      content,
      receiver_id: receiverId,
      group_id: groupId,
      message_type: 'text'
    })
  });
  
  // Add to local messages
  const chatKey = groupId ? `group_${groupId}` : `contact_${receiverId}`;
  if (!messages[chatKey]) {
    messages[chatKey] = [];
  }
  messages[chatKey].push(data);
  
  // Send via WebSocket for real-time delivery
  sendWebSocketMessage({
    event_type: 'message',
    user_id: currentUser.id,
    receiver_id: receiverId,
    group_id: groupId,
    content: content
  });
  
  return data;
}

async function getMessages(receiverId = null, groupId = null) {
  const params = new URLSearchParams();
  if (receiverId) params.append('receiver_id', receiverId);
  if (groupId) params.append('group_id', groupId);
  
  const data = await apiCall(`/messages/list?${params}`);
  
  const chatKey = groupId ? `group_${groupId}` : `contact_${receiverId}`;
  messages[chatKey] = data;
  
  return data;
}

// Groups
async function createGroup(name, description) {
  const data = await apiCall('/groups/create', {
    method: 'POST',
    body: JSON.stringify({ name, description })
  });
  
  groups.push(data);
  renderGroups();
  
  return data;
}

async function getUserGroups() {
  const data = await apiCall('/groups/list');
  groups = data;
  renderGroups();
  return data;
}

// UI Rendering
function renderContacts() {
  const contactList = document.getElementById('contact-list');
  contactList.innerHTML = '';
  
  // For demo purposes, add some mock contacts
  const mockContacts = [
    { id: 1, username: '张三', status: 'online' },
    { id: 2, username: '李四', status: 'offline' },
    { id: 3, username: '王五', status: 'away' }
  ];
  
  mockContacts.forEach(contact => {
    const contactItem = document.createElement('div');
    contactItem.className = 'contact-item';
    contactItem.onclick = () => selectChat('contact', contact.id, contact.username);
    
    contactItem.innerHTML = `
      <div class="avatar">${contact.username.charAt(0)}</div>
      <div class="contact-info">
        <div class="contact-name">${contact.username}</div>
        <div class="last-message">点击开始聊天</div>
      </div>
    `;
    
    contactList.appendChild(contactItem);
  });
}

function renderGroups() {
  const groupList = document.getElementById('group-list');
  groupList.innerHTML = '<button class="create-group-btn" id="create-group-btn">+ 创建群组</button>';
  
  groups.forEach(group => {
    const groupItem = document.createElement('div');
    groupItem.className = 'group-item';
    groupItem.onclick = () => selectChat('group', group.id, group.name);
    
    groupItem.innerHTML = `
      <div class="avatar">${group.name.charAt(0)}</div>
      <div class="group-info">
        <div class="group-name">${group.name}</div>
        <div class="last-message">${group.description || '点击开始聊天'}</div>
      </div>
    `;
    
    groupList.appendChild(groupItem);
  });
  
  // Re-attach create group button handler
  document.getElementById('create-group-btn').onclick = () => {
    document.getElementById('create-group-modal').classList.remove('hidden');
  };
}

async function selectChat(type, id, name) {
  currentChat = { type, id, name };
  
  // Update header
  document.getElementById('chat-header').innerHTML = `
    <div class="chat-title">${name}</div>
  `;
  
  // Show message input
  document.getElementById('message-input-container').classList.remove('hidden');
  
  // Load messages
  if (type === 'contact') {
    await getMessages(id, null);
  } else {
    await getMessages(null, id);
  }
  
  renderMessages();
  
  // Update active state
  document.querySelectorAll('.contact-item, .group-item').forEach(item => {
    item.classList.remove('active');
  });
  event.currentTarget.classList.add('active');
}

function renderMessages() {
  const messageList = document.getElementById('message-list');
  
  if (!currentChat) {
    messageList.innerHTML = '<div class="no-chat-selected"><p>请选择一个联系人或群组开始聊天</p></div>';
    return;
  }
  
  const chatKey = currentChat.type === 'group' 
    ? `group_${currentChat.id}` 
    : `contact_${currentChat.id}`;
  
  const chatMessages = messages[chatKey] || [];
  
  if (chatMessages.length === 0) {
    messageList.innerHTML = '<div class="no-chat-selected"><p>暂无消息，开始聊天吧！</p></div>';
    return;
  }
  
  messageList.innerHTML = '';
  
  chatMessages.forEach(msg => {
    const isSent = msg.sender_id === currentUser.id;
    const messageDiv = document.createElement('div');
    messageDiv.className = `message ${isSent ? 'sent' : 'received'}`;
    
    const time = new Date(msg.created_at).toLocaleTimeString('zh-CN', { 
      hour: '2-digit', 
      minute: '2-digit' 
    });
    
    messageDiv.innerHTML = `
      <div class="avatar">${isSent ? currentUser.username.charAt(0) : currentChat.name.charAt(0)}</div>
      <div class="message-content">
        <div class="message-bubble">${msg.content}</div>
        <div class="message-time">${time}</div>
      </div>
    `;
    
    messageList.appendChild(messageDiv);
  });
  
  // Scroll to bottom
  messageList.scrollTop = messageList.scrollHeight;
}

async function handleSendMessage() {
  const input = document.getElementById('message-input');
  const content = input.value.trim();
  
  if (!content || !currentChat) return;
  
  try {
    if (currentChat.type === 'contact') {
      await sendMessage(content, currentChat.id, null);
    } else {
      await sendMessage(content, null, currentChat.id);
    }
    
    input.value = '';
    renderMessages();
  } catch (error) {
    alert('发送消息失败: ' + error.message);
  }
}

// Event Listeners
window.addEventListener('DOMContentLoaded', () => {
  // Check for saved auth
  const savedToken = localStorage.getItem('authToken');
  const savedUser = localStorage.getItem('currentUser');
  
  if (savedToken && savedUser) {
    authToken = savedToken;
    currentUser = JSON.parse(savedUser);
    showPage('main-page');
    document.getElementById('current-username').textContent = currentUser.username;
    connectWebSocket();
    renderContacts();
    getUserGroups();
  } else {
    showPage('login-page');
  }
  
  // Login form
  document.getElementById('login-form').onsubmit = async (e) => {
    e.preventDefault();
    const email = document.getElementById('login-email').value;
    const password = document.getElementById('login-password').value;
    
    try {
      await login(email, password);
      showPage('main-page');
      document.getElementById('current-username').textContent = currentUser.username;
      connectWebSocket();
      renderContacts();
      getUserGroups();
    } catch (error) {
      alert('登录失败: ' + error.message);
    }
  };
  
  // Register form
  document.getElementById('register-form').onsubmit = async (e) => {
    e.preventDefault();
    const username = document.getElementById('register-username').value;
    const email = document.getElementById('register-email').value;
    const password = document.getElementById('register-password').value;
    
    try {
      await register(username, email, password);
      showPage('main-page');
      document.getElementById('current-username').textContent = currentUser.username;
      connectWebSocket();
      renderContacts();
      getUserGroups();
    } catch (error) {
      alert('注册失败: ' + error.message);
    }
  };
  
  // Show register/login links
  document.getElementById('show-register').onclick = (e) => {
    e.preventDefault();
    showPage('register-page');
  };
  
  document.getElementById('show-login').onclick = (e) => {
    e.preventDefault();
    showPage('login-page');
  };
  
  // Tabs
  document.querySelectorAll('.tab').forEach(tab => {
    tab.onclick = () => {
      document.querySelectorAll('.tab').forEach(t => t.classList.remove('active'));
      tab.classList.add('active');
      
      const tabType = tab.dataset.tab;
      if (tabType === 'contacts') {
        document.getElementById('contact-list').classList.remove('hidden');
        document.getElementById('group-list').classList.add('hidden');
      } else {
        document.getElementById('contact-list').classList.add('hidden');
        document.getElementById('group-list').classList.remove('hidden');
      }
    };
  });
  
  // Send message
  document.getElementById('send-btn').onclick = handleSendMessage;
  
  document.getElementById('message-input').onkeypress = (e) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSendMessage();
    }
  };
  
  // Create group modal
  document.getElementById('create-group-form').onsubmit = async (e) => {
    e.preventDefault();
    const name = document.getElementById('group-name').value;
    const description = document.getElementById('group-description').value;
    
    try {
      await createGroup(name, description);
      document.getElementById('create-group-modal').classList.add('hidden');
      document.getElementById('create-group-form').reset();
    } catch (error) {
      alert('创建群组失败: ' + error.message);
    }
  };
  
  // Close modal
  document.querySelector('.close').onclick = () => {
    document.getElementById('create-group-modal').classList.add('hidden');
  };
  
  // Close modal on outside click
  window.onclick = (e) => {
    const modal = document.getElementById('create-group-modal');
    if (e.target === modal) {
      modal.classList.add('hidden');
    }
  };
});

