import { useState, useEffect } from "react";
import Login from "./components/Login";
import Dashboard from "./components/Dashboard";
import "./App.css";

interface UserInfo {
  email: String;
  name: String;
  picture: string | null;
  is_admin: boolean;
}

function App() {
  const [user, setUser] = useState<UserInfo | null>(null);
  const [theme, setTheme] = useState<'dark' | 'light'>(() => {
    return (localStorage.getItem("theme") as 'dark' | 'light') || 'dark';
  });

  useEffect(() => {
    if (theme === 'light') {
      document.body.classList.add('light-theme');
    } else {
      document.body.classList.remove('light-theme');
    }
    localStorage.setItem("theme", theme);
  }, [theme]);

  const toggleTheme = () => {
    setTheme((prev) => (prev === 'dark' ? 'light' : 'dark'));
  };

  if (!user) {
    return (
      <div style={{ position: "relative" }}>
        {/* Floating Theme Toggle on Login Screen */}
        <button 
          className="sign-out-btn" 
          style={{ position: "absolute", top: "1rem", right: "1rem", zIndex: 1000, padding: "0.5rem" }}
          onClick={toggleTheme}
        >
          {theme === 'dark' ? "☀️ Light" : "🌙 Dark"}
        </button>
        <Login onLoginSuccess={(loggedInUser) => setUser(loggedInUser)} />
      </div>
    );
  }

  return (
    <Dashboard 
      user={user} 
      onSignOut={() => setUser(null)} 
      theme={theme}
      onToggleTheme={toggleTheme}
    />
  );
}

export default App;
