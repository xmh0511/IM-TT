import { useState, useEffect, useRef } from 'react';
import { User, Message, Group } from '../types';
import { apiService } from '../services/api';
import { wsService } from '../services/websocket';
import './Chat.css';

interface ChatProps {
  user: User;
  onLogout: () => void;
}

export default function Chat({ user, onLogout }: ChatProps) {
  const [messages, setMessages] = useState<Message[]>([]);
  const [groups, setGroups] = useState<Group[]>([]);
  const [currentChat, setCurrentChat] = useState<{ type: 'contact' | 'group'; id: number; name: string } | null>(null);
  const [messageInput, setMessageInput] = useState('');
  const messagesEndRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    // Load user groups
    apiService.getUserGroups().then(setGroups);

    // Listen for WebSocket messages
    const handleWsMessage = (event: any) => {
      if (event.event_type === 'message') {
        // Add message to state
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

    wsService.addListener(handleWsMessage);
    return () => wsService.removeListener(handleWsMessage);
  }, []);

  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [messages]);

  const loadMessages = async (type: 'contact' | 'group', id: number) => {
    const msgs = type === 'contact'
      ? await apiService.getMessages(id)
      : await apiService.getMessages(undefined, id);
    setMessages(msgs);
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
      
      // Send via WebSocket
      wsService.send({
        event_type: 'message',
        user_id: user.id,
        receiver_id: receiverId || undefined,
        group_id: groupId || undefined,
        content: messageInput,
      });

      setMessageInput('');
      loadMessages(currentChat.type, currentChat.id);
    } catch (error) {
      console.error('Failed to send message:', error);
    }
  };

  return (
    <div className="chat-container">
      <div className="sidebar">
        <div className="user-info">
          <div className="avatar">{user.username.charAt(0)}</div>
          <span>{user.username}</span>
          <button onClick={onLogout}>退出</button>
        </div>
        
        <div className="groups-list">
          <h3>群组</h3>
          {groups.map(group => (
            <div
              key={group.id}
              className={`chat-item ${currentChat?.id === group.id ? 'active' : ''}`}
              onClick={() => handleSelectChat('group', group.id, group.name)}
            >
              <div className="avatar">{group.name.charAt(0)}</div>
              <span>{group.name}</span>
            </div>
          ))}
        </div>
      </div>

      <div className="chat-area">
        {currentChat ? (
          <>
            <div className="chat-header">
              <h2>{currentChat.name}</h2>
            </div>
            
            <div className="messages-list">
              {messages.map(msg => (
                <div
                  key={msg.id}
                  className={`message ${msg.sender_id === user.id ? 'sent' : 'received'}`}
                >
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
          <div className="no-chat">请选择一个聊天</div>
        )}
      </div>
    </div>
  );
}
