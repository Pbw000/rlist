function escapeHtml(e){var t=document.createElement("div");return t.textContent=e,t.innerHTML}function getFileIcon(e){return`<i class="ti ${{pdf:"ti-file-text",doc:"ti-file-text",docx:"ti-file-text",xls:"ti-file-spreadsheet",xlsx:"ti-file-spreadsheet",ppt:"ti-file-presentation",pptx:"ti-file-presentation",jpg:"ti-file-image",jpeg:"ti-file-image",png:"ti-file-image",gif:"ti-file-image",svg:"ti-file-image",webp:"ti-file-image",bmp:"ti-file-image",ico:"ti-file-image",mp3:"ti-file-music",wav:"ti-file-music",flac:"ti-file-music",aac:"ti-file-music",ogg:"ti-file-music",mp4:"ti-file-video",avi:"ti-file-video",mkv:"ti-file-video",mov:"ti-file-video",webm:"ti-file-video",zip:"ti-file-zip",rar:"ti-file-zip","7z":"ti-file-zip",tar:"ti-file-zip",gz:"ti-file-zip",txt:"ti-file-text",md:"ti-file-text",log:"ti-file-text",js:"ti-file-code",ts:"ti-file-code",py:"ti-file-code",java:"ti-file-code",cpp:"ti-file-code",c:"ti-file-code",go:"ti-file-code",rs:"ti-file-code",html:"ti-file-code",css:"ti-file-code",json:"ti-file-code",xml:"ti-file-code",yaml:"ti-file-code",yml:"ti-file-code",sh:"ti-file-code"}[e.split(".").pop().toLowerCase()]||"ti-file"}"></i>`}function formatSize(e){var t;return 0===e?"0 B":(t=Math.floor(Math.log(e)/Math.log(1024)),parseFloat((e/Math.pow(1024,t)).toFixed(1))+" "+["B","KB","MB","GB","TB"][t])}function formatDate(e){try{return new Date(e).toLocaleDateString("zh-CN")}catch{return e}}function showToast(e,t="info"){let i=document.createElement("div");i.className="toast "+t,i.innerHTML=`<i class="${"success"===t?"ti ti-check":"error"===t?"ti ti-alert-circle":"ti ti-info-circle"}"></i><span>${e}</span>`,document.body.appendChild(i),setTimeout(()=>{i.remove()},3e3)}function toggleTheme(){var e="dark"===document.documentElement.getAttribute("data-theme"),t=document.getElementById("themeIcon");e?(document.documentElement.removeAttribute("data-theme"),t&&(t.className="ti ti-moon"),localStorage.setItem("rlist_theme","light")):(document.documentElement.setAttribute("data-theme","dark"),t&&(t.className="ti ti-sun"),localStorage.setItem("rlist_theme","dark"))}function setView(e){localStorage.setItem("rlist_view",e);var t=document.getElementById("fileList"),i=document.getElementById("listViewBtn"),a=document.getElementById("gridViewBtn");"grid"===e?(t?.classList.add("grid-view"),i?.classList.remove("active"),a?.classList.add("active")):(t?.classList.remove("grid-view"),i?.classList.add("active"),a?.classList.remove("active"))}async function calculateFileHash(e){e=await e.arrayBuffer(),e=await crypto.subtle.digest("SHA-256",e);return Array.from(new Uint8Array(e)).map(e=>e.toString(16).padStart(2,"0")).join("")}function parseHash(e){if(e&&"empty"!==e&&"object"==typeof e){if(e.sha256)return{algo:"sha256",value:e.sha256};if(e.md5)return{algo:"md5",value:e.md5}}return null}function formatHash(e,t="sha256"){return e?{[t]:e}:"empty"}async function copyToClipboard(e){try{return await navigator.clipboard.writeText(e),!0}catch{var t=document.createElement("textarea");t.value=e,t.style.position="fixed",t.style.left="-999999px",document.body.appendChild(t),t.select();try{return document.execCommand("copy"),document.body.removeChild(t),!0}catch{return document.body.removeChild(t),!1}}}async function sha512(e){e=await crypto.subtle.digest("SHA-512",e);return Array.from(new Uint8Array(e)).map(e=>e.toString(16).padStart(2,"0")).join("")}function stringToBytes(e){return(new TextEncoder).encode(e)}function bigIntToBigEndianBytes(t){var i=new Uint8Array(8);for(let e=7;0<=e;e--)i[e]=Number(0xffn&t),t>>=8n;return i}function mergeBytes(...e){var t,i=e.reduce((e,t)=>e+t.length,0),a=new Uint8Array(i);let n=0;for(t of e)a.set(t,n),n+=t.length;return a}async function getChallenge(){try{var e,t,i,a,n=await fetch("/api/challenge",{method:"GET",headers:{"Content-Type":"application/json"}});if(n.ok)return(t=(e=await n.text()).match(/"salt"\s*:\s*(\d+)/))?(i=BigInt(t[1]),200===(a=JSON.parse(e)).code&&a.data?{success:!0,salt:i}:{success:!1,message:a.message||"获取 Challenge 失败"}):{success:!1,message:"获取 Challenge 失败：无法解析 salt"};throw new Error(`HTTP ${n.status}: `+n.statusText)}catch(e){return{success:!1,message:"网络错误："+e.message}}}function generateRandomNonce(){var e=new Uint8Array(16);return crypto.getRandomValues(e),Array.from(e).map(e=>e.toString(16).padStart(2,"0")).join("")}async function buildPublicRequest(e,t=3){var i,a,n,r=await getChallenge();if(r.success)return{nonce:t,claim:n}=await computeChallengeInWorker(i=r.salt,a=Math.floor(Date.now()/1e3),e.path||"",t),{...e,salt:i.toString(),timestamp:a,nonce:t,claim:n};throw new Error(r.message)}function computeChallengeInWorker(n,r,o,s){return new Promise((t,i)=>{var e=new Blob([`
      self.onmessage = async function(e) {
        const { salt, timestamp, path, difficulty } = e.data;

        // SHA512 实现
        async function sha512(data) {
          const hashBuffer = await crypto.subtle.digest("SHA-512", data);
          const hashArray = Array.from(new Uint8Array(hashBuffer));
          return hashArray.map((b) => b.toString(16).padStart(2, "0")).join("");
        }

        // 字符串转字节
        function stringToBytes(str) {
          return new TextEncoder().encode(str);
        }

        // BigInt 转大端序字节
        function bigIntToBigEndianBytes(num) {
          const bytes = new Uint8Array(8);
          for (let i = 7; i >= 0; i--) {
            bytes[i] = Number(num & 0xffn);
            num = num >> 8n;
          }
          return bytes;
        }

        // 合并字节
        function mergeBytes(...arrays) {
          const totalLength = arrays.reduce((sum, arr) => sum + arr.length, 0);
          const result = new Uint8Array(totalLength);
          let offset = 0;
          for (const arr of arrays) {
            result.set(arr, offset);
            offset += arr.length;
          }
          return result;
        }

        // 生成随机 nonce
        function generateRandomNonce() {
          const array = new Uint8Array(16);
          crypto.getRandomValues(array);
          return Array.from(array).map((b) => b.toString(16).padStart(2, "0")).join("");
        }

        const saltHex = salt.toString(16);
        const pathBytes = stringToBytes(path);
        const timestampBytes = bigIntToBigEndianBytes(BigInt(timestamp));
        const saltHexBytes = stringToBytes(saltHex);
        const targetZeros = "0".repeat(difficulty);

        let nonce = generateRandomNonce();
        let nonceBytes = stringToBytes(nonce);
        let combinedData = mergeBytes(nonceBytes, pathBytes, timestampBytes, saltHexBytes);
        let claim = await sha512(combinedData);

        // 寻找满足难度的 nonce
        while (!claim.startsWith(targetZeros)) {
          nonce = generateRandomNonce();
          nonceBytes = stringToBytes(nonce);
          combinedData = mergeBytes(nonceBytes, pathBytes, timestampBytes, saltHexBytes);
          claim = await sha512(combinedData);
        }

        self.postMessage({ nonce, claim });
      };
    `],{type:"application/javascript"});let a=new Worker(URL.createObjectURL(e));a.onmessage=e=>{t(e.data),a.terminate()},a.onerror=e=>{i(e),a.terminate()},a.postMessage({salt:n,timestamp:r,path:o,difficulty:s})})}