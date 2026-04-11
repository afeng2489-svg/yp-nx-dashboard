// Content Script - 与页面脚本和background通信

// 监听页面消息
window.addEventListener('message', (event) => {
  if (event.data.type === 'NX_CHOOSE_DIRECTORY') {
    chrome.runtime.sendMessage(
      { action: 'chooseDirectory' },
      (response) => {
        if (response.error) {
          window.postMessage({
            type: 'NX_DIRECTORY_ERROR',
            error: response.error
          }, '*');
        } else {
          window.postMessage({
            type: 'NX_DIRECTORY_SELECTED',
            directory: response.result
          }, '*');
        }
      }
    );
  }

  if (event.data.type === 'NX_GET_RETAINED_DIRECTORY') {
    chrome.runtime.sendMessage(
      { action: 'getRetainedEntry' },
      (response) => {
        if (response) {
          window.postMessage({
            type: 'NX_RETAINED_DIRECTORY',
            directory: response
          }, '*');
        } else {
          window.postMessage({
            type: 'NX_RETAINED_DIRECTORY',
            directory: null
          }, '*');
        }
      }
    );
  }
});