// Chrome FileSystem API - Background Service Worker

// 选择文件夹
function chooseDirectory(callback) {
  chrome.fileSystem.chooseEntry(
    { type: 'openDirectory' },
    (entry) => {
      if (chrome.runtime.lastError) {
        console.error('选择文件夹错误:', chrome.runtime.lastError);
        callback(null, chrome.runtime.lastError.message);
        return;
      }

      if (!entry) {
        callback(null, '用户取消选择');
        return;
      }

      // 保存引用以便后续使用
      chrome.storage.local.set({
        retainedEntry: chrome.fileSystem.retainEntry(entry)
      });

      callback({
        fullPath: entry.fullPath,
        name: entry.name,
        isDirectory: entry.isDirectory
      });
    }
  );
}

// 读取目录内容
function readDirectory(entry, callback) {
  const reader = entry.createReader();
  const entries = [];

  reader.readEntries((results) => {
    if (chrome.runtime.lastError) {
      callback(null, chrome.runtime.lastError.message);
      return;
    }

    results.forEach((e) => {
      entries.push({
        name: e.name,
        fullPath: e.fullPath,
        isDirectory: e.isDirectory,
        isFile: e.isFile
      });
    });

    callback(entries);
  });
}

// 读取文件
function readFile(entry, callback) {
  entry.file((file) => {
    file.arrayBuffer().then((buffer) => {
      callback(null, buffer);
    }).catch((err) => {
      callback(err.message);
    });
  }, (err) => {
    callback(err.message);
  });
}

// 恢复保留的访问权限
function restoreEntry(callback) {
  chrome.storage.local.get(['retainedEntry'], ({ retainedEntry }) => {
    if (!retainedEntry) {
      callback(null, '没有保存的访问权限');
      return;
    }

    chrome.fileSystem.restoreEntry(retainedEntry, (entry) => {
      if (chrome.runtime.lastError) {
        callback(null, chrome.runtime.lastError.message);
        return;
      }
      callback(entry);
    });
  });
}

// 处理来自 content script 的消息
chrome.runtime.onMessage.addListener((request, sender, sendResponse) => {
  switch (request.action) {
    case 'chooseDirectory':
      chooseDirectory((result, error) => {
        sendResponse({ result, error });
      });
      return true; // 异步响应

    case 'readDirectory':
      restoreEntry((entry, error) => {
        if (error || !entry) {
          sendResponse({ error });
          return;
        }
        readDirectory(entry, sendResponse);
      });
      return true;

    case 'readFile':
      restoreEntry((entry, error) => {
        if (error || !entry) {
          sendResponse({ error });
          return;
        }
        readFile(entry, sendResponse);
      });
      return true;

    case 'getRetainedEntry':
      chrome.storage.local.get(['retainedEntry'], ({ retainedEntry }) => {
        if (retainedEntry) {
          chrome.fileSystem.restoreEntry(retainedEntry, (entry) => {
            if (entry) {
              sendResponse({
                fullPath: entry.fullPath,
                name: entry.name
              });
            } else {
              sendResponse(null);
            }
          });
        } else {
          sendResponse(null);
        }
      });
      return true;

    default:
      sendResponse({ error: '未知操作' });
  }
});