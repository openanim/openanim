import { useState, useEffect, useRef } from 'react';
import { 
  Send, Cpu, Sparkles, Settings, ChevronDown, ChevronUp, 
  Video, Download, CheckCircle2, Terminal, RefreshCw, X, 
  MessageSquare, AlertCircle
} from 'lucide-react';
import './App.css';

// TypeScript Types matching scene_ir JSON schema
interface RGBA {
  r: number;
  g: number;
  b: number;
  a: number;
}

interface Vector2D {
  x: number;
  y: number;
}

interface ShapeKind {
  type: string;
  radius?: number;
  width?: number;
  height?: number;
}

interface ShapeComponent {
  kind: ShapeKind;
}

interface StyleComponent {
  fill?: RGBA;
  stroke?: {
    color: RGBA;
    width: number;
    line_cap?: string;
    line_join?: string;
  };
  opacity?: number;
  z_index?: number;
  visible?: boolean;
}

interface TransformComponent {
  position: Vector2D;
  scale: Vector2D;
  rotation: number;
}

interface TextComponent {
  text: string;
  font_size: number;
  color: RGBA;
  font_family?: string;
}

interface NodeComponents {
  shape?: ShapeComponent;
  style?: StyleComponent;
  transform?: TransformComponent;
  text?: TextComponent;
}

interface SceneNode {
  id: string;
  name: string;
  node_type: string;
  parent: string | null;
  children: string[];
  components: NodeComponents;
}

interface KeyframeValue {
  type: string;
  value: number;
}

interface Keyframe {
  time: number;
  value: KeyframeValue;
  easing: string;
}

interface Track {
  target_node: string;
  property: string;
  keyframes: Keyframe[];
}

interface SceneTimeline {
  duration: number;
  tracks: Track[];
  events: any[];
}

interface Scene {
  id: string;
  name: string;
  root_node: string;
  nodes: SceneNode[];
  timeline: SceneTimeline;
  duration: number;
  metadata: any;
}

interface StatusResponse {
  engine_version: string;
  adapters: {
    ffmpeg: boolean;
    manim: boolean;
    mermaid: boolean;
    remotion: boolean;
  };
}

interface LlmSettings {
  providerType: 'openai' | 'anthropic' | 'ollama';
  apiKey: string;
  baseUrl: string;
  model: string;
}

interface ChatMessage {
  id: string;
  sender: 'user' | 'assistant';
  text: string;
  status: 'sending' | 'thinking' | 'rendering' | 'success' | 'error';
  scene?: Scene;
  videoUrl?: string;
  errorMsg?: string;
  timestamp: Date;
}

export default function App() {
  // App Core States
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [inputText, setInputText] = useState<string>('');
  const [activeSceneId, setActiveSceneId] = useState<string>('');
  const [systemStatus, setSystemStatus] = useState<StatusResponse | null>(null);
  
  // Settings Dialog toggle and values
  const [showSettings, setShowSettings] = useState<boolean>(false);
  const [llmSettings, setLlmSettings] = useState<LlmSettings>(() => {
    const saved = localStorage.getItem('openanim_llm_settings');
    if (saved) {
      try {
        return JSON.parse(saved);
      } catch (e) {
        console.error("Failed to parse saved LLM settings", e);
      }
    }
    return {
      providerType: 'openai',
      apiKey: '',
      baseUrl: 'https://api.openai.com/v1',
      model: 'gpt-4o-mini'
    };
  });

  // UI States
  const [toast, setToast] = useState<{ message: string; type: 'success' | 'error' | 'info' } | null>(null);
  const [expandedCodeMessageId, setExpandedCodeMessageId] = useState<string>('');

  const chatEndRef = useRef<HTMLDivElement | null>(null);

  // Save Settings to localStorage whenever they change
  useEffect(() => {
    localStorage.setItem('openanim_llm_settings', JSON.stringify(llmSettings));
  }, [llmSettings]);

  // Load Status and default messages on mount
  useEffect(() => {
    fetchStatus();
    loadWelcomeMessages();
  }, []);

  // Auto-scroll chat feed
  useEffect(() => {
    chatEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [messages]);

  const showToast = (message: string, type: 'success' | 'error' | 'info' = 'success') => {
    setToast({ message, type });
    setTimeout(() => setToast(null), 4000);
  };

  const fetchStatus = async () => {
    try {
      const res = await fetch('/api/status');
      if (res.ok) {
        const data = await res.json();
        setSystemStatus(data);
      }
    } catch (e) {
      console.error("Failed to fetch system capability status", e);
    }
  };

  const loadWelcomeMessages = () => {
    setMessages([
      {
        id: 'welcome-1',
        sender: 'assistant',
        text: "Hello! I am your **OpenAnim AI Studio**. You can prompt me to build and render high-fidelity animations entirely from natural language. \n\nNo manual keyframing or track management needed! Just describe what you want, and I'll generate the Rust Scene IR, compile the animations, and play the output video right here.",
        status: 'success',
        timestamp: new Date()
      }
    ]);
  };

  const handleProviderChange = (type: 'openai' | 'anthropic' | 'ollama') => {
    let baseUrl = 'https://api.openai.com/v1';
    let model = 'gpt-4o-mini';

    if (type === 'anthropic') {
      baseUrl = 'https://api.anthropic.com/v1';
      model = 'claude-3-5-sonnet-20241022';
    } else if (type === 'ollama') {
      baseUrl = 'http://localhost:11434';
      model = 'llama3';
    }

    setLlmSettings(prev => ({
      ...prev,
      providerType: type,
      baseUrl,
      model,
      apiKey: type === 'ollama' ? 'local-run' : prev.apiKey
    }));
  };

  // Maps our frontend LlmSettings format to the backend LlmProvider serialization structure
  const buildLlmProviderPayload = () => {
    const { providerType, apiKey, baseUrl, model } = llmSettings;
    
    if (providerType === 'openai') {
      return {
        provider_type: 'open_ai',
        api_key: apiKey || 'demo-key',
        model: model ? model : null,
        base_url: baseUrl ? baseUrl : null
      };
    } else if (providerType === 'anthropic') {
      return {
        provider_type: 'anthropic',
        api_key: apiKey || 'demo-key',
        model: model ? model : null
      };
    } else {
      return {
        provider_type: 'ollama',
        base_url: baseUrl || 'http://localhost:11434',
        model: model || 'llama3'
      };
    }
  };

  const handleSendPrompt = async (textToSend?: string) => {
    const prompt = textToSend ? textToSend.trim() : inputText.trim();
    if (!prompt) return;

    if (!textToSend) {
      setInputText('');
    }

    // Check key requirements for non-Ollama providers
    if (llmSettings.providerType !== 'ollama' && !llmSettings.apiKey) {
      setShowSettings(true);
      showToast("Please enter your API key to configure your LLM Provider first!", "error");
      return;
    }

    const messageId = Math.random().toString(36).substring(7);
    const userMessage: ChatMessage = {
      id: `user-${messageId}`,
      sender: 'user',
      text: prompt,
      status: 'success',
      timestamp: new Date()
    };

    const assistantMessage: ChatMessage = {
      id: `assistant-${messageId}`,
      sender: 'assistant',
      text: '',
      status: 'thinking',
      timestamp: new Date()
    };

    setMessages(prev => [...prev, userMessage, assistantMessage]);

    try {
      const isPatch = activeSceneId !== '';
      const endpoint = isPatch ? '/api/patch' : '/api/generate';
      const providerPayload = buildLlmProviderPayload();

      const requestPayload = isPatch 
        ? { prompt, provider: providerPayload, scene_id: activeSceneId }
        : { prompt, provider: providerPayload };

      // Phase 1: Call LLM Compiler
      const compilerRes = await fetch(endpoint, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(requestPayload)
      });

      if (!compilerRes.ok) {
        const errorText = await compilerRes.text();
        throw new Error(`LLM compilation error: ${errorText}`);
      }

      const sceneData: Scene = await compilerRes.json();
      setActiveSceneId(sceneData.id);

      // Update message with Scene IR and transit to rendering step
      setMessages(prev => prev.map(msg => 
        msg.id === assistantMessage.id 
          ? { 
              ...msg, 
              status: 'rendering', 
              scene: sceneData, 
              text: `I've successfully ${isPatch ? 'patched and updated' : 'architected'} the animation scene **"${sceneData.name}"**! Now compiling and rendering the video file...` 
            } 
          : msg
      ));

      // Phase 2: Call Render Orchestrator
      const renderRes = await fetch('/api/render', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ scene_id: sceneData.id })
      });

      if (!renderRes.ok) {
        const errorText = await renderRes.text();
        throw new Error(`Orchestrator render failed: ${errorText}`);
      }

      const renderData = await renderRes.json();
      const videoUrl = `/api/video/${renderData.video_hash}`;

      // Update message to success state with streaming url
      setMessages(prev => prev.map(msg => 
        msg.id === assistantMessage.id 
          ? { 
              ...msg, 
              status: 'success', 
              videoUrl,
              text: `Here is the rendered animation for **"${sceneData.name}"**! Built cleanly in 100% Rust and rendered using the high-fidelity engine.`
            } 
          : msg
      ));

    } catch (err: any) {
      console.error(err);
      setMessages(prev => prev.map(msg => 
        msg.id === assistantMessage.id 
          ? { 
              ...msg, 
              status: 'error', 
              errorMsg: err.message || "An unexpected error occurred during generation."
            } 
          : msg
      ));
    }
  };

  const handleClearContext = () => {
    setActiveSceneId('');
    loadWelcomeMessages();
    showToast("Animation session context reset. Starting a fresh scene!", "info");
  };

  const promptSuggestions = [
    "Draw a spinning teal circle that glides to the right edge with ease-in-out easing over 5s.",
    "Write 'OpenAnim' as a glowing text that scales up in the center of the screen.",
    "Create a warning animation with a flashing orange rectangle and alert text.",
    "Add a smooth fade-in and slide-up animation for a group of rectangular blocks."
  ];

  return (
    <div className="studio-container">
      {/* Sleek Top Header Navigation */}
      <header className="studio-header glass-panel">
        <div style={{ display: 'flex', alignItems: 'center', gap: 12 }}>
          <div className="dot-glow dot-blue animate-pulse" style={{ width: 12, height: 12 }} />
          <h1 style={{ fontSize: '1.2rem', margin: 0, fontWeight: 700 }} className="text-gradient">
            OpenAnim Studio
          </h1>
          <span style={{ fontSize: '0.7rem', padding: '2px 8px', borderRadius: 4, backgroundColor: 'rgba(255,255,255,0.06)', color: 'var(--text-secondary)' }}>
            100% Rust Engine
          </span>
        </div>

        {/* Engine Capability Status */}
        <div className="header-status-area">
          <div className={`adapter-pill ${systemStatus?.adapters.ffmpeg ? 'active' : ''}`}>
            <span className={`dot-glow ${systemStatus?.adapters.ffmpeg ? 'dot-green' : 'dot-red'}`} />
            FFmpeg
          </div>
          <div className={`adapter-pill ${systemStatus?.adapters.manim ? 'active' : ''}`}>
            <span className={`dot-glow ${systemStatus?.adapters.manim ? 'dot-green' : 'dot-red'}`} />
            Manim
          </div>
          <div className={`adapter-pill ${systemStatus?.adapters.remotion ? 'active' : ''}`}>
            <span className={`dot-glow ${systemStatus?.adapters.remotion ? 'dot-green' : 'dot-red'}`} />
            Remotion
          </div>
        </div>

        {/* Workspace controls */}
        <div style={{ display: 'flex', gap: 10 }}>
          {activeSceneId && (
            <button className="control-btn clear-context-btn" onClick={handleClearContext}>
              <RefreshCw size={14} style={{ marginRight: 6 }} />
              Reset Context
            </button>
          )}
          <button className="control-btn" style={{ gap: 6 }} onClick={() => setShowSettings(true)}>
            <Settings size={15} />
            LLM Settings
          </button>
        </div>
      </header>

      {/* Main Chat Layout Area */}
      <div className="chat-layout-main">
        <div className="chat-stream-container">
          <div className="chat-feed">
            {messages.map((msg) => (
              <div key={msg.id} className={`chat-message-row ${msg.sender}`}>
                <div className={`chat-bubble ${msg.sender === 'user' ? 'user-bubble' : 'assistant-bubble'}`}>
                  {/* Sender Icon */}
                  <div className="bubble-sender-title">
                    {msg.sender === 'user' ? (
                      <span className="sender-pill user-pill">You</span>
                    ) : (
                      <span className="sender-pill assistant-pill">
                        <Sparkles size={11} style={{ marginRight: 4 }} />
                        OpenAnim Assistant
                      </span>
                    )}
                  </div>

                  {/* Message Text Content */}
                  <div className="bubble-content-text">
                    {msg.text.split('\n\n').map((paragraph, index) => (
                      <p key={index} style={{ marginBottom: 12, lineHeight: 1.5, fontSize: '0.92rem' }}>
                        {paragraph}
                      </p>
                    ))}
                  </div>

                  {/* Loading/Thinking states */}
                  {msg.status === 'thinking' && (
                    <div className="thinking-indicator-area glass-panel">
                      <Cpu size={16} className="spinner text-gradient" />
                      <span className="pulse-text">Architecting Scene Graph & IR Keyframes...</span>
                    </div>
                  )}

                  {msg.status === 'rendering' && (
                    <div className="thinking-indicator-area glass-panel">
                      <Video size={16} className="spinner text-gradient" />
                      <span className="pulse-text">Compiling tracks & rendering high-fidelity MP4...</span>
                    </div>
                  )}

                  {/* Error State */}
                  {msg.status === 'error' && (
                    <div className="error-indicator-card">
                      <div className="error-title">
                        <AlertCircle size={16} style={{ color: 'var(--accent-red)' }} />
                        <span>Compilation Failed</span>
                      </div>
                      <p className="error-msg-body">{msg.errorMsg}</p>
                      <button className="control-btn retry-btn" onClick={() => handleSendPrompt(messages.find(m => m.id === msg.id.replace('assistant-', 'user-'))?.text)}>
                        <RefreshCw size={12} style={{ marginRight: 6 }} />
                        Retry Request
                      </button>
                    </div>
                  )}

                  {/* Inline Video Player on Success */}
                  {msg.status === 'success' && msg.videoUrl && (
                    <div className="inline-video-previewer glass-panel">
                      <video 
                        src={msg.videoUrl} 
                        controls 
                        autoPlay 
                        loop 
                        className="chat-embedded-video"
                      />
                      <div className="video-meta-bar">
                        <div style={{ display: 'flex', gap: 12, fontSize: '0.75rem', color: 'var(--text-secondary)' }}>
                          <span>Duration: <strong>{msg.scene?.duration || '5.0'}s</strong></span>
                          <span>FPS: <strong>60</strong></span>
                          <span>Format: <strong>MP4 (H.264)</strong></span>
                        </div>
                        <a 
                          href={msg.videoUrl} 
                          download={`${msg.scene?.name || 'animation'}.mp4`}
                          className="control-btn download-video-btn"
                          style={{ padding: '4px 10px', fontSize: '0.75rem' }}
                        >
                          <Download size={12} style={{ marginRight: 4 }} />
                          Download MP4
                        </a>
                      </div>
                    </div>
                  )}

                  {/* Collapsible Scene IR Code block */}
                  {msg.scene && (
                    <div className="scene-ir-container">
                      <button 
                        className="code-toggle-btn"
                        onClick={() => setExpandedCodeMessageId(expandedCodeMessageId === msg.id ? '' : msg.id)}
                      >
                        <Terminal size={12} style={{ marginRight: 6 }} />
                        <span>{expandedCodeMessageId === msg.id ? 'Hide Rust Scene IR' : 'View Rust Scene IR JSON'}</span>
                        {expandedCodeMessageId === msg.id ? <ChevronUp size={12} style={{ marginLeft: 'auto' }} /> : <ChevronDown size={12} style={{ marginLeft: 'auto' }} />}
                      </button>
                      
                      {expandedCodeMessageId === msg.id && (
                        <pre className="scene-ir-pre scrollbar">
                          <code className="scrollbar">{JSON.stringify(msg.scene, null, 2)}</code>
                        </pre>
                      )}
                    </div>
                  )}
                </div>
              </div>
            ))}
            <div ref={chatEndRef} />
          </div>
        </div>

        {/* Bottom Input Area and Suggestions */}
        <div className="chat-input-sticky-footer">
          {/* Active scene context indicator */}
          {activeSceneId && (
            <div className="context-indicator-bar glass-panel">
              <span className="dot-glow dot-green animate-pulse" />
              <span style={{ fontSize: '0.78rem', color: 'var(--text-secondary)' }}>
                Continuous context active. Next prompt will **modify / patch** the active scene structure.
              </span>
              <button className="context-reset-link" onClick={handleClearContext}>
                Reset to start fresh
              </button>
            </div>
          )}

          {/* Quick Starter Suggestions (only shows if history is short) */}
          {messages.length <= 1 && (
            <div className="starter-suggestions-grid">
              {promptSuggestions.map((sug, i) => (
                <button key={i} className="suggestion-card glass-panel" onClick={() => handleSendPrompt(sug)}>
                  <MessageSquare size={13} style={{ color: 'var(--accent-blue)', flexShrink: 0 }} />
                  <span className="suggestion-text">{sug}</span>
                </button>
              ))}
            </div>
          )}

          {/* Chat text box */}
          <div className="chat-input-glass-container glass-panel">
            <textarea
              className="chat-input-textarea"
              placeholder={activeSceneId ? "Modify scene (e.g. 'now make the ball red and speed up rotation')..." : "Describe an animation to compile (e.g. 'Draw a spinning blue star glide left')..."}
              value={inputText}
              onChange={(e) => setInputText(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === 'Enter' && !e.shiftKey) {
                  e.preventDefault();
                  handleSendPrompt();
                }
              }}
            />
            <button 
              className="btn-glowing send-prompt-btn" 
              onClick={() => handleSendPrompt()}
              disabled={!inputText.trim()}
              style={{ width: 40, height: 40, minHeight: 40, borderRadius: '50%', justifyContent: 'center', padding: 0 }}
            >
              <Send size={15} />
            </button>
          </div>
        </div>
      </div>

      {/* Floating Settings Drawer Modal */}
      {showSettings && (
        <div className="settings-drawer-backdrop" onClick={() => setShowSettings(false)}>
          <div className="settings-drawer glass-panel" onClick={(e) => e.stopPropagation()}>
            <div className="drawer-header">
              <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                <Settings size={16} className="text-gradient" />
                <h2 style={{ fontSize: '1.05rem', margin: 0, fontWeight: 700 }}>LLM Provider Settings</h2>
              </div>
              <button className="control-btn" style={{ padding: 4 }} onClick={() => setShowSettings(false)}>
                <X size={16} />
              </button>
            </div>

            <div className="drawer-body">
              <p style={{ fontSize: '0.8rem', color: 'var(--text-secondary)', lineHeight: 1.4, marginBottom: 16 }}>
                OpenAnim Studio queries LLM providers directly and securely from your local engine. Your keys are stored locally in your browser.
              </p>

              {/* Provider Selection */}
              <div className="form-group">
                <label className="form-label">LLM Provider</label>
                <div className="provider-select-tabs">
                  <button 
                    className={`provider-tab ${llmSettings.providerType === 'openai' ? 'active' : ''}`}
                    onClick={() => handleProviderChange('openai')}
                  >
                    OpenAI
                  </button>
                  <button 
                    className={`provider-tab ${llmSettings.providerType === 'anthropic' ? 'active' : ''}`}
                    onClick={() => handleProviderChange('anthropic')}
                  >
                    Anthropic
                  </button>
                  <button 
                    className={`provider-tab ${llmSettings.providerType === 'ollama' ? 'active' : ''}`}
                    onClick={() => handleProviderChange('ollama')}
                  >
                    Ollama (Local)
                  </button>
                </div>
              </div>

              {/* API Key */}
              {llmSettings.providerType !== 'ollama' && (
                <div className="form-group">
                  <label className="form-label">API Key</label>
                  <input 
                    type="password"
                    className="form-input"
                    placeholder={`Enter your ${llmSettings.providerType === 'openai' ? 'OpenAI' : 'Anthropic'} API key`}
                    value={llmSettings.apiKey}
                    onChange={(e) => setLlmSettings(prev => ({ ...prev, apiKey: e.target.value }))}
                  />
                </div>
              )}

              {/* API Base URL */}
              <div className="form-group">
                <label className="form-label">Base URL / Endpoint</label>
                <input 
                  type="text"
                  className="form-input"
                  placeholder="e.g. https://api.openai.com/v1"
                  value={llmSettings.baseUrl}
                  onChange={(e) => setLlmSettings(prev => ({ ...prev, baseUrl: e.target.value }))}
                />
                <span className="field-hint-text">
                  {llmSettings.providerType === 'openai' && "Default: https://api.openai.com/v1 (or custom OpenAI compatible endpoints)"}
                  {llmSettings.providerType === 'anthropic' && "Default: https://api.anthropic.com/v1"}
                  {llmSettings.providerType === 'ollama' && "Default: http://localhost:11434 (make sure Ollama is running locally)"}
                </span>
              </div>

              {/* Model Selection */}
              <div className="form-group">
                <label className="form-label">LLM Model Name</label>
                <input 
                  type="text"
                  className="form-input"
                  placeholder="e.g. gpt-4o-mini"
                  value={llmSettings.model}
                  onChange={(e) => setLlmSettings(prev => ({ ...prev, model: e.target.value }))}
                />
                <span className="field-hint-text">
                  {llmSettings.providerType === 'openai' && "Recommended: gpt-4o-mini or gpt-4o"}
                  {llmSettings.providerType === 'anthropic' && "Recommended: claude-3-5-sonnet-20241022"}
                  {llmSettings.providerType === 'ollama' && "Recommended: llama3, qwen2.5-coder:7b or similar"}
                </span>
              </div>
            </div>

            <div className="drawer-footer">
              <button className="btn-glowing" style={{ width: '100%', justifyContent: 'center' }} onClick={() => setShowSettings(false)}>
                Save LLM Configurations
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Modern Top Toast notifications */}
      {toast && (
        <div className="toast slide-in">
          <CheckCircle2 size={16} style={{ color: toast.type === 'error' ? 'var(--accent-red)' : 'var(--accent-emerald)' }} />
          <span style={{ fontSize: '0.85rem' }}>{toast.message}</span>
        </div>
      )}
    </div>
  );
}
