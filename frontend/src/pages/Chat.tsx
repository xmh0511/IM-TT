import { useState, useEffect, useRef, useCallback } from 'react';
import { User, Message, Group } from '../types';
import { apiService } from '../services/api';
import { wsService } from '../services/websocket';
import './Chat.css';

interface ChatProps {
  user: User;
  onLogout: () => void;
}

type ChatTarget = { type: 'contact' | 'group'; id: number; name: string };

export default function Chat({ user, onLogout }: ChatProps) {
  const [contacts, setContacts] = useState<User[]>([]);
  const [groups, setGroups] = useState<Group[]>([]);
  const [messages, setMessages] = useState<Message[]>([]);
  const [currentChat, setCurrentChat] = useState<ChatTarget | null>(null);
  const [messageInput, setMessageInput] = useState('');
  const [activeTab, setActiveTab] = useState<'contacts' | 'groups'>('contacts');
  const [onlineUsers, setOnlineUsers] = useState<Set<number>>(new Set());
  const [showCreateGroup, setShowCreateGroup] = useState(false);
  const [showJoinGroup, setShowJoinGroup] = useState(false);
  const [newGroupName, setNewGroupName] = useState('');
  const [newGroupDesc, setNewGroupDesc] = useState('');
  const [joinGroupId, setJoinGroupId] = useState('');
  const messagesEndRef = useRef<HTMLDivElement>(null);

  const loadData = useCallback(async () => {
    try {
      const [usersList, groupsList] = await Promise.all([
        apiService.getAllUsers(),
        apiService.getUserGroups(),
      ]);
      setContacts(usersList);
      setGroups(groupsList);
    } catch (error) {
      console.error('Failed to load data:', error);
    }
  }, []);

  useEffect(() => {
    loadData();

    const handleWsMessage = (event: any) => {
      if (event.event_type === 'message') {
        setMessages(prev => [...prev, {
          id: Date.now(),
          sender_id: event.user_id,
          receiver_id: event.receiver_id,
          group_id: event.group_id,
          content: event.content,
          message_type: 'text',
          created_at: new Date().toISOString(),
          is_read: false,
        }]);
      }
    };

    const handleOnlineUsers = (users: Set<number>) => {
      setOnlineUsers(new Set(users));
    };

    wsService.addListener(handleWsMessage);
    wsService.addOnlineListener(handleOnlineUsers);

    return () => {
      wsService.removeListener(handleWsMessage);
      wsService.removeOnlineListener(handleOnlineUsers);
    };
  }, [loadData]);

  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [messages]);

  const loadMessages = async (type: 'contact' | 'group', id: number) => {
    try {
      const msgs = type === 'contact'
        ? await apiService.getMessages(id)
        : await apiService.getMessages(undefined, id);
      setMessages(msgs);
    } catch (error) {
      console.error('Failed to load messages:', error);
    }
  };

  const handleSelectChat = (type: 'contact' | 'group', id: number, name: string) => {
    setCurrentChat({ type, id, name });
    loadMessages(type, id);
  };

  const handleSendMessage = async () => {
    if (!messageInput.trim() || !currentChat) return;

    try {
      const receiverId = currentChat.type === 'contact' ? currentChat.id : null;
      const groupId = currentChat.type === 'group' ? currentChat.id : null;

      await apiService.sendMessage(receiverId, groupId, messageInput);

      wsService.send({
        event_type: 'message',
        user_id: user.id,
        receiver_id: receiverId || undefined,
        group_id: groupId || undefined,
        content: messageInput,
      });

      setMessages(prev => [...prev, {
        id: Date.now(),
        sender_id: user.id,
        receiver_id: receiverId || undefined,
        group_id: groupId || undefined,
        content: messageInput,
        message_type: 'text',
        created_at: new Date().toISOString(),
        is_read: false,
      }]);

      setMessageInput('');
    } catch (error) {
      console.error('Failed to send message:', error);
    }
  };

  const handleCreateGroup = async () => {
    if (!newGroupName.trim()) return;
    try {
      await apiService.createGroup(newGroupName, newGroupDesc || undefined);
      setShowCreateGroup(false);
      setNewGroupName('');
      setNewGroupDesc('');
      loadData();
    } catch (error: any) {
      alert(error.message || 'Failed to create group');
    }
  };

  const handleJoinGroup = async () => {
    if (!joinGroupId.trim()) return;
    try {
      await apiService.joinGroup(parseInt(joinGroupId));
      setShowJoinGroup(false);
      setJoinGroupId('');
      loadData();
    } catch (error: any) {
      alert(error.message || 'Failed to join group');
    }
  };

  const handleLogout = async () => {
    try {
      await apiService.logout();
    } catch {
      apiService.clearToken();
    }
    wsService.disconnect();
    onLogout();
  };

  const getContactStatus = (contactId: number) => {
    return onlineUsers.has(contactId);
  };

  return (
    <div className="chat-container">
      <div className="sidebar">
        <div className="user-info">
          <div className="avatar">{user.username.charAt(0).toUpperCase()}</div>
          <span className="username">{user.username}</span>
          <button className="logout-btn" onClick={handleLogout}>退出</button>
        </div>

        <div className="tab-bar">
          <button
            className={`tab ${activeTab === 'contacts' ? 'active' : ''}`}
            onClick={() => setActiveTab('contacts')}
          >
            联系人
          </button>
          <button
            className={`tab ${activeTab === 'groups' ? 'active' : ''}`}
            onClick={() => setActiveTab('groups')}
          >
            群组
          </button>
        </div>

        <div className="list-container">
          {activeTab === 'contacts' && (
            <div className="contacts-list">
              <div className="list-header">
                <span>好友列表 ({contacts.length})</span>
              </div>
              {contacts.map(contact => (
                <div
                  key={contact.id}
                  className={`chat-item ${currentChat?.id === contact.id && currentChat?.type === 'contact' ? 'active' : ''}`}
                  onClick={() => handleSelectChat('contact', contact.id, contact.username)}
                >
                  <div className="avatar-wrapper">
                    <div className="avatar">{contact.username.charAt(0).toUpperCase()}</div>
                    <span className={`status-dot ${getContactStatus(contact.id) ? 'online' : ''}`}></span>
                  </div>
                  <div className="item-info">
                    <span className="item-name">{contact.username}</span>
                    <span className="item-status">{getContactStatus(contact.id) ? '在线' : '离线'}</span>
                  </div>
                </div>
              ))}
              {contacts.length === 0 && (
                <div className="empty-hint">暂无联系人</div>
              )}
            </div>
          )}

          {activeTab === 'groups' && (
            <div className="groups-list">
              <div className="list-header">
                <span>我的群组 ({groups.length})</span>
                <div className="list-actions">
                  <button className="action-btn" onClick={() => setShowCreateGroup(true)} title="创建群组">+</button>
                  <button className="action-btn" onClick={() => setShowJoinGroup(true)} title="加入群组">入</button>
                </div>
              </div>
              {groups.map(group => (
                <div
                  key={group.id}
                  className={`chat-item ${currentChat?.id === group.id && currentChat?.type === 'group' ? 'active' : ''}`}
                  onClick={() => handleSelectChat('group', group.id, group.name)}
                >
                  <div className="avatar">{group.name.charAt(0).toUpperCase()}</div>
                  <div className="item-info">
                    <span className="item-name">{group.name}</span>
                    <span className="item-status">{group.description || '暂无描述'}</span>
                  </div>
                </div>
              ))}
              {groups.length === 0 && (
                <div className="empty-hint">暂无群组，点击 + 创建或入 加入</div>
              )}
            </div>
          )}
        </div>
      </div>

      <div className="chat-area">
        {currentChat ? (
          <>
            <div className="chat-header">
              <div className="avatar">{currentChat.name.charAt(0).toUpperCase()}</div>
              <div className="chat-header-info">
                <h2>{currentChat.name}</h2>
                <span className="chat-header-status">
                  {currentChat.type === 'group' ? '群聊' : (getContactStatus(currentChat.id) ? '在线' : '离线')}
                </span>
              </div>
            </div>

            <div className="messages-list">
              {messages.map(msg => (
                <div
                  key={msg.id}
                  className={`message ${msg.sender_id === user.id ? 'sent' : 'received'}`}
                >
                  {currentChat.type === 'group' && msg.sender_id !== user.id && (
                    <div className="message-sender">
                      {contacts.find(c => c.id === msg.sender_id)?.username || `User ${msg.sender_id}`}
                    </div>
                  )}
                  <div className="message-content">{msg.content}</div>
                  <div className="message-time">
                    {new Date(msg.created_at).toLocaleTimeString()}
                  </div>
                </div>
              ))}
              <div ref={messagesEndRef} />
            </div>

            <div className="message-input">
              <input
                type="text"
                value={messageInput}
                onChange={(e) => setMessageInput(e.target.value)}
                onKeyPress={(e) => e.key === 'Enter' && handleSendMessage()}
                placeholder="输入消息..."
              />
              <button onClick={handleSendMessage}>发送</button>
            </div>
          </>
        ) : (
          <div className="no-chat">
            <div className="no-chat-icon">IM</div>
            <p>选择一个聊天开始对话</p>
          </div>
        )}
      </div>

      {showCreateGroup && (
        <div className="modal-overlay" onClick={() => setShowCreateGroup(false)}>
          <div className="modal" onClick={e => e.stopPropagation()}>
            <h3>创建群组</h3>
            <input
              type="text"
              placeholder="群组名称"
              value={newGroupName}
              onChange={e => setNewGroupName(e.target.value)}
            />
            <input
              type="text"
              placeholder="群组描述 (可选)"
              value={newGroupDesc}
              onChange={e => setNewGroupDesc(e.target.value)}
            />
            <div className="modal-actions">
              <button className="cancel-btn" onClick={() => setShowCreateGroup(false)}>取消</button>
              <button className="confirm-btn" onClick={handleCreateGroup}>创建</button>
            </div>
          </div>
        </div>
      )}

      {showJoinGroup && (
        <div className="modal-overlay" onClick={() => setShowJoinGroup(false)}>
          <div className="modal" onClick={e => e.stopPropagation()}>
            <h3>加入群组</h3>
            <input
              type="text"
              placeholder="输入群组 ID"
              value={joinGroupId}
              onChange={e => setJoinGroupId(e.target.value)}
            />
            <div className="modal-actions">
              <button className="cancel-btn" onClick={() => setShowJoinGroup(false)}>取消</button>
              <button className="confirm-btn" onClick={handleJoinGroup}>加入</button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
