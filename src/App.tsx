import { useState, useEffect } from "react";
import "./App.css";

interface Printer {
  name: string;
  is_default: boolean;
}

function App() {
  const [status, setStatus] = useState("Checking...");
  const [printers, setPrinters] = useState<Printer[]>([]);
  const [loading, setLoading] = useState(false);

  const AGENT_URL = "http://localhost:18088";

  // 1. 检查 Agent 是否存活
  const checkHealth = async () => {
    try {
      const res = await fetch(`${AGENT_URL}/`);
      const text = await res.text();
      setStatus(text);
    } catch (e) {
      setStatus("Agent offline (Connection Failed)");
    }
  };

  // 2. 获取打印机列表
  const fetchPrinters = async () => {
    try {
      const res = await fetch(`${AGENT_URL}/printers`);
      const data = await res.json();
      setPrinters(data);
    } catch (e) {
      console.error(e);
    }
  };

  // 3. 发送测试打印 (生成 PDF)
  const testPrint = async () => {
    setLoading(true);
    try {
      const res = await fetch(`${AGENT_URL}/print`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          task_id: crypto.randomUUID(),
          content: "Hello from React Web Interface!",
        }),
      });
      const result = await res.json();
      alert(`Agent Response: ${result.message}\nPath: ${result.debug_path}`);
    } catch (e) {
      alert("Print failed");
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    checkHealth();
  }, []);

  return (
    <div className="container">
      <h1>DeepPrint Agent</h1>
      
      <div className="card">
        <h3>Status: <span style={{color: status.includes("Running") ? "green" : "red"}}>{status}</span></h3>
        <button onClick={fetchPrinters}>Refresh Printers</button>
      </div>

      <div className="card">
        <h3>Local Printers:</h3>
        {printers.length === 0 ? <p>No printers found or not fetched.</p> : (
          <ul style={{textAlign: 'left'}}>
            {printers.map((p) => (
              <li key={p.name}>
                {p.name} {p.is_default && <strong>(Default)</strong>}
              </li>
            ))}
          </ul>
        )}
      </div>

      <div className="card">
        <h3>Test Engine</h3>
        <p>This will generate a PDF on your desktop via Skia.</p>
        <button onClick={testPrint} disabled={loading}>
          {loading ? "Rendering..." : "Generate Test PDF"}
        </button>
      </div>
    </div>
  );
}

export default App;