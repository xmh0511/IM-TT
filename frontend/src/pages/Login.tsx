import { useState } from 'react';
import { User } from '../types';
import { apiService } from '../services/api';
import './Login.css';

interface LoginProps {
  onLogin: (user: User) => void;
}

export default function Login({ onLogin }: LoginProps) {
  const [isRegister, setIsRegister] = useState(false);
  const [username, setUsername] = useState('');
  const [email, setEmail] = useState('');
  const [password, setPassword] = useState('');
  const [error, setError] = useState('');
  const [loading, setLoading] = useState(false);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError('');
    setLoading(true);

    try {
      const response = isRegister
        ? await apiService.register(username, email, password)
        : await apiService.login(email, password);
      
      onLogin(response.user);
    } catch (err: any) {
      setError(err.message || '操作失败');
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="login-container">
      <div className="login-box">
        <h1>IM-TT 即时通讯</h1>
        <h2>{isRegister ? '注册' : '登录'}</h2>
        
        <form onSubmit={handleSubmit}>
          {isRegister && (
            <input
              type="text"
              placeholder="用户名"
              value={username}
              onChange={(e) => setUsername(e.target.value)}
              required
            />
          )}
          
          <input
            type="email"
            placeholder="邮箱"
            value={email}
            onChange={(e) => setEmail(e.target.value)}
            required
          />
          
          <input
            type="password"
            placeholder="密码"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            required
          />
          
          {error && <div className="error">{error}</div>}
          
          <button type="submit" disabled={loading}>
            {loading ? '处理中...' : (isRegister ? '注册' : '登录')}
          </button>
        </form>
        
        <p>
          {isRegister ? '已有账号？' : '还没有账号？'}
          <button
            className="link-button"
            onClick={() => setIsRegister(!isRegister)}
          >
            {isRegister ? '立即登录' : '立即注册'}
          </button>
        </p>
      </div>
    </div>
  );
}
