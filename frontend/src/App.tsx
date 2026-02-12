import { useState, useEffect } from 'react';
import { User } from './types';
import { apiService } from './services/api';
import { wsService } from './services/websocket';
import Login from './pages/Login';
import Chat from './pages/Chat';
import './App.css';

function App() {
  const [user, setUser] = useState<User | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    const token = apiService.getToken();
    if (token) {
      apiService.getCurrentUser()
        .then(userData => {
          setUser(userData);
          wsService.connect(token);
        })
        .catch(() => {
          apiService.clearToken();
        })
        .finally(() => setLoading(false));
    } else {
      setLoading(false);
    }
  }, []);

  const handleLogin = (userData: User) => {
    setUser(userData);
    const token = apiService.getToken();
    if (token) {
      wsService.connect(token);
    }
  };

  const handleLogout = () => {
    wsService.disconnect();
    apiService.clearToken();
    setUser(null);
  };

  if (loading) {
    return <div className="loading">加载中...</div>;
  }

  return (
    <div className="app">
      {user ? (
        <Chat user={user} onLogout={handleLogout} />
      ) : (
        <Login onLogin={handleLogin} />
      )}
    </div>
  );
}

export default App;
