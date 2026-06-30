import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";

interface UserInfo {
  email: String;
  name: String;
  picture: string | null;
  is_admin: boolean;
}

interface LoginProps {
  onLoginSuccess: (user: UserInfo) => void;
}

export default function Login({ onLoginSuccess }: LoginProps) {
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleLogin = async () => {
    setLoading(true);
    setError(null);
    try {
      const user = await invoke<UserInfo>("login_with_google");
      onLoginSuccess(user);
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="login-screen">
      <div className="login-bg-logo" />
      <div className="login-card">
        <h1 className="login-logo">NeuroKidsDoc DB</h1>
        <h2 className="login-title">Database Authentication</h2>
        <p className="login-desc">
          Sign in using your corporate Google Workspace email ending with <strong>@neuropedia.ae</strong> or <strong>@cdcdubai.ae</strong>.
        </p>

        {error && (
          <div className="login-status error">
            <strong>Login Failed:</strong> {error}
          </div>
        )}

        {loading ? (
          <div className="login-status loading">
            <div style={{ display: "flex", alignItems: "center", gap: "0.5rem" }}>
              <div className="spinner" style={{ width: "16px", height: "16px", borderWidth: "2px" }}></div>
              <span>Waiting for Google authentication in your browser...</span>
            </div>
          </div>
        ) : (
          <button className="google-login-btn" onClick={handleLogin}>
            <svg className="google-icon" viewBox="0 0 24 24">
              <path
                fill="#4285F4"
                d="M22.56 12.25c0-.78-.07-1.53-.2-2.25H12v4.26h5.92c-.26 1.37-1.04 2.53-2.21 3.31v2.77h3.57c2.08-1.92 3.28-4.74 3.28-8.09z"
              />
              <path
                fill="#34A853"
                d="M12 23c2.97 0 5.46-.98 7.28-2.66l-3.57-2.77c-.98.66-2.23 1.06-3.71 1.06-2.86 0-5.29-1.93-6.16-4.53H2.18v2.84C3.99 20.53 7.7 23 12 23z"
              />
              <path
                fill="#FBBC05"
                d="M5.84 14.09c-.22-.66-.35-1.36-.35-2.09s.13-1.43.35-2.09V7.06H2.18C1.43 8.55 1 10.22 1 12s.43 3.45 1.18 4.94l2.85-2.22.81-.63z"
              />
              <path
                fill="#EA4335"
                d="M12 5.38c1.62 0 3.06.56 4.21 1.64l3.15-3.15C17.45 2.09 14.97 1 12 1 7.7 1 3.99 3.47 2.18 7.06l3.66 2.84c.87-2.6 3.3-4.52 6.16-4.52z"
              />
            </svg>
            Sign in with Google
          </button>
        )}
      </div>
    </div>
  );
}
