#!/usr/bin/env node

const { spawn } = require('child_process');
const path = require('path');

// 构建可执行文件路径
const exePath = path.join(__dirname, 'target', 'debug', 'bilidl.exe');

// 启动MCP服务器
const server = spawn(exePath, ['--mcp'], {
    stdio: ['pipe', 'pipe', 'inherit']
});

let requestId = 1;

// 发送初始化请求
function sendInitialize() {
    const request = {
        jsonrpc: "2.0",
        id: requestId++,
        method: "initialize",
        params: {
            protocolVersion: "2024-11-05",
            capabilities: {},
            clientInfo: {
                name: "test-client",
                version: "1.0.0"
            }
        }
    };

    console.log('发送初始化请求:', JSON.stringify(request, null, 2));
    server.stdin.write(JSON.stringify(request) + '\n');
}

// 发送工具列表请求
function sendToolsList() {
    const request = {
        jsonrpc: "2.0",
        id: requestId++,
        method: "tools/list",
        params: {}
    };

    console.log('发送工具列表请求:', JSON.stringify(request, null, 2));
    server.stdin.write(JSON.stringify(request) + '\n');
}

// 监听服务器输出
server.stdout.on('data', (data) => {
    const response = data.toString().trim();
    console.log('收到响应:', response);

    try {
        const parsed = JSON.parse(response);
        if (parsed.id === 1) {
            // 初始化响应，发送工具列表请求
            setTimeout(sendToolsList, 100);
        }
    } catch (e) {
        console.error('解析响应失败:', e);
    }
});

server.on('close', (code) => {
    console.log(`服务器退出，退出码: ${code}`);
});

server.on('error', (err) => {
    console.error('启动服务器失败:', err);
});

// 启动测试
console.log('启动MCP服务器测试...');
sendInitialize();

// 10秒后退出
setTimeout(() => {
    console.log('测试完成，关闭服务器...');
    server.kill();
}, 10000);