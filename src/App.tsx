import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";

// 定义类型以匹配 Rust 结构
interface PrinterDto {
  name: String;
  system_name: String;
  is_default: boolean;
}

function App() {
  const [printers, setPrinters] = useState<PrinterDto[]>([]);

  // 调用 Rust 后端获取打印机
  const refreshPrinters = async () => {
    try {
      const list = await invoke<PrinterDto[]>("agent_get_printers");
      setPrinters(list);
    } catch (e) {
      console.error("获取打印机失败", e);
    }
  };

  useEffect(() => {
    refreshPrinters();
  }, []);

  return (
    <div className="container" style={{padding: '20px', fontFamily: 'sans-serif', maxWidth: '800px', margin: '0 auto'}}>
      <h1 style={{borderBottom: '2px solid #eee', paddingBottom: '10px'}}>🖨️ DeepPrint Agent 控制台</h1>
      
      <div style={{background: '#e0f2f1', padding: '15px', borderRadius: '8px', marginBottom: '20px', color: '#00695c'}}>
        <p style={{margin: '5px 0'}}>✅ <strong>服务状态：</strong> 运行中</p>
        <p style={{margin: '5px 0'}}>🌐 <strong>监听接口：</strong> <a href="http://localhost:18088/printers" target="_blank" style={{color: '#00695c'}}>http://localhost:18088</a></p>
      </div>

      <div style={{display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '10px'}}>
        <h3>本机打印机列表 ({printers.length})</h3>
        <button onClick={refreshPrinters} style={{padding: '8px 16px', cursor: 'pointer'}}>刷新列表</button>
      </div>
      
      <ul style={{listStyle: 'none', padding: 0, border: '1px solid #eee', borderRadius: '8px'}}>
        {printers.map((p, idx) => (
          <li key={idx} style={{padding: '15px', borderBottom: idx < printers.length -1 ? '1px solid #eee' : 'none', display: 'flex', justifyContent: 'space-between', alignItems: 'center'}}>
            <div>
              <span style={{fontWeight: 'bold', fontSize: '1.1em'}}>{p.name}</span>
              <div style={{fontSize: '0.8em', color: '#666'}}>{p.system_name}</div>
            </div>
            <button onClick={() => alert(`测试任务已发送至: ${p.name}`)} style={{padding: '6px 12px', fontSize: '0.9em', cursor: 'pointer'}}>测试打印</button>
          </li>
        ))}
        {printers.length === 0 && <li style={{padding: '20px', textAlign: 'center', color: '#999'}}>未检测到打印机</li>}
      </ul>
    </div>
  );
}

export default App;