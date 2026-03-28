let API_BASE="/obs",currentPath="/",filesData=[],currentView=localStorage.getItem("rlist_public_view")||"list",previewFilePath="",contextMenuTarget=null,previewZoom=1,previewRotation=0,previewPdfDoc=null,previewFileType="",pathHistory=[],pathHistoryIndex=-1,isNavigatingHistory=!1,currentCursor=null,hasMorePages=!1,isLoadingMore=!1,PAGE_SIZE=20;function initTheme(){var e;"dark"===(localStorage.getItem("rlist_theme")||"light")&&(document.documentElement.setAttribute("data-theme","dark"),e=document.getElementById("themeIcon"))&&(e.className="ti ti-sun")}function toggleTheme(){var e="dark"===document.documentElement.getAttribute("data-theme"),t=document.getElementById("themeIcon");e?(document.documentElement.removeAttribute("data-theme"),t&&(t.className="ti ti-moon"),localStorage.setItem("rlist_theme","light")):(document.documentElement.setAttribute("data-theme","dark"),t&&(t.className="ti ti-sun"),localStorage.setItem("rlist_theme","dark"))}function setView(e){currentView=e,localStorage.setItem("rlist_view",e);var t=document.getElementById("fileList"),i=document.getElementById("listViewBtn"),a=document.getElementById("gridViewBtn");"grid"===e?(t?.classList.add("grid-view"),i?.classList.remove("active"),a?.classList.add("active")):(t?.classList.remove("grid-view"),i?.classList.add("active"),a?.classList.remove("active"))}async function loadFiles(t=currentPath,i=!0){currentPath=t;t=document.getElementById("fileList");if(t){if(i)currentCursor=null,hasMorePages=!1,filesData=[],t.innerHTML='<div class="loading"><div class="spinner"></div></div>';else{if(isLoadingMore)return;isLoadingMore=!0;var e=t.querySelector(".loading-more");e?e.style.display="flex":((e=document.createElement("div")).className="loading-more",e.innerHTML='<div class="spinner spinner-small"></div><span>加载中...</span>',t.appendChild(e))}try{var a,n,o={path:currentPath,per_page:PAGE_SIZE},r=(null!==currentCursor&&(o.cursor=currentCursor),await buildPublicRequest(o)),l=await(await fetch(API_BASE+"/list",{method:"POST",headers:{"Content-Type":"application/json"},body:JSON.stringify(r)})).json();200===l.code&&l.data?(a=(l.data.items||[]).map(e=>{var t=void 0===e.File;return{name:e.File?.name||e.Directory?.name||"unknown",path:currentPath.endsWith("/")?currentPath+(e.File?.name||e.Directory?.name||""):currentPath+"/"+(e.File?.name||e.Directory?.name||""),size:e.File?.size||0,file_type:t?"dir":"file",modified:e.File?.modified_at||e.Directory?.modified_at}}),currentCursor=l.data.next_cursor,hasMorePages=null!=currentCursor,i?renderFiles(filesData=a):renderFiles(filesData=[...filesData,...a],!1),(n=t.querySelector(".loading-more"))&&n.remove(),updateBreadcrumb(),isLoadingMore=!1):(i&&(t.innerHTML='<div class="empty-state"><i class="ti ti-folder-x"></i><p>加载失败</p></div>'),showToast("加载文件列表失败："+l.message,"error"))}catch(e){i&&(t.innerHTML='<div class="empty-state"><i class="ti ti-wifi-off"></i><p>无法连接到服务器</p></div>'),showToast("网络错误："+e.message,"error"),isLoadingMore=!1}}}async function loadMoreFiles(){hasMorePages&&!isLoadingMore&&await loadFiles(currentPath,!1)}function renderFiles(e,t=!0){var i,a=document.getElementById("fileList");a&&(e&&0!==e.length?(i=e.filter(e=>"dir"===e.file_type),e=e.filter(e=>"file"===e.file_type),i=[...i,...e],t?(a.innerHTML=`
      <div class="file-list-header">
          <div>名称</div>
          <div>大小</div>
          <div>修改日期</div>
          <div>操作</div>
      </div>
      ${i.map(e=>`
          <div class="file-item"
               data-path="${escapeHtml(e.path)}"
               data-type="${e.file_type}"
               oncontextmenu="showContextMenu(event, '${escapeHtml(e.path)}', '${e.file_type}')">
              <div class="file-main" onclick="handleFileClick('${escapeHtml(e.path)}', '${e.file_type}', event)">
                  <div class="file-icon ${"dir"===e.file_type?"folder":""}">${"dir"===e.file_type?'<i class="ti ti-folder"></i>':getFileIcon(e.name)}</div>
                  <div>
                      <div class="file-name">${escapeHtml(e.name)}</div>
                      <div class="file-meta">${"file"===e.file_type?formatSize(e.size):"文件夹"}</div>
                  </div>
              </div>
              <div class="file-size">${"file"===e.file_type?formatSize(e.size):""}</div>
              <div class="file-date">${e.modified?formatDate(e.modified):""}</div>
              <div class="file-actions">
                  ${"dir"===e.file_type?`
                      <button class="action-btn" onclick="enterFolderWithAnimation('${escapeHtml(e.path)}')" title="打开">
                          <i class="ti ti-folder-open"></i>
                      </button>
                  `:`
                      <button class="action-btn" onclick="previewFile('${escapeHtml(e.path)}', '${escapeHtml(e.name)}')" title="预览">
                          <i class="ti ti-eye"></i>
                      </button>
                      <button class="action-btn" onclick="downloadFile('${escapeHtml(e.path)}')" title="下载">
                          <i class="ti ti-download"></i>
                      </button>
                  `}
                  <button class="action-btn" onclick="showContextMenuForFile('${escapeHtml(e.path)}', '${e.file_type}')" title="更多">
                      <i class="ti ti-dots"></i>
                  </button>
              </div>
          </div>
      `).join("")}
    `,setupScrollListener()):(e=i.map(e=>`
        <div class="file-item"
             data-path="${escapeHtml(e.path)}"
             data-type="${e.file_type}"
             oncontextmenu="showContextMenu(event, '${escapeHtml(e.path)}', '${e.file_type}')">
            <div class="file-main" onclick="handleFileClick('${escapeHtml(e.path)}', '${e.file_type}', event)">
                <div class="file-icon ${"dir"===e.file_type?"folder":""}">${"dir"===e.file_type?'<i class="ti ti-folder"></i>':getFileIcon(e.name)}</div>
                <div>
                    <div class="file-name">${escapeHtml(e.name)}</div>
                    <div class="file-meta">${"file"===e.file_type?formatSize(e.size):"文件夹"}</div>
                </div>
            </div>
            <div class="file-size">${"file"===e.file_type?formatSize(e.size):""}</div>
            <div class="file-date">${e.modified?formatDate(e.modified):""}</div>
            <div class="file-actions">
                ${"dir"===e.file_type?`
                    <button class="action-btn" onclick="enterFolderWithAnimation('${escapeHtml(e.path)}')" title="打开">
                        <i class="ti ti-folder-open"></i>
                    </button>
                `:`
                    <button class="action-btn" onclick="previewFile('${escapeHtml(e.path)}', '${escapeHtml(e.name)}')" title="预览">
                        <i class="ti ti-eye"></i>
                    </button>
                    <button class="action-btn" onclick="downloadFile('${escapeHtml(e.path)}')" title="下载">
                        <i class="ti ti-download"></i>
                    </button>
                `}
                <button class="action-btn" onclick="showContextMenuForFile('${escapeHtml(e.path)}', '${e.file_type}')" title="更多">
                    <i class="ti ti-dots"></i>
                </button>
            </div>
        </div>
    `).join(""),(t=a.querySelector(".loading-more"))?t.insertAdjacentHTML("beforebegin",e):a.insertAdjacentHTML("beforeend",e))):a.innerHTML='<div class="empty-state"><i class="ti ti-folder-open"></i><p>此目录为空</p></div>')}function setupScrollListener(){var e=document.querySelector(".file-list-container");e&&(e.removeEventListener("scroll",handleScroll),e.addEventListener("scroll",handleScroll))}function handleScroll(){var e,t,i=document.querySelector(".file-list-container");i&&({scrollTop:i,scrollHeight:e,clientHeight:t}=i,e-100<=i+t)&&loadMoreFiles()}function handleDoubleClick(e,t){"dir"===t?enterFolder(e):previewFile(e,e.split("/").pop())}function handleFileClick(e,t,i){i.target.closest(".file-actions")||"dir"===t&&enterFolderWithAnimation(e)}function enterFolderWithAnimation(e){let t=document.getElementById("fileList");t&&(t.classList.add("nav-entering"),navigateTo(e),setTimeout(()=>{t.classList.remove("nav-entering")},400))}function updateBreadcrumb(){var e=document.getElementById("breadcrumb");if(e){let i=currentPath.split("/").filter(e=>e),a='<a href="#" onclick="navigateTo(\'/\'); return false;" class="breadcrumb-item"><i class="ti ti-home"></i><span>首页</span></a>',n="";i.forEach((e,t)=>{n+="/"+e;t=t===i.length-1;a=(a+='<span class="breadcrumb-separator">/</span>')+(t?`<a href="#" onclick="navigateTo('${escapeHtml(n)}'); return false;" class="breadcrumb-item active">
        <span>${escapeHtml(e)}</span>
      </a>`:`<a href="#" onclick="navigateTo('${escapeHtml(n)}'); return false;" class="breadcrumb-item">
        <span>${escapeHtml(e)}</span>
      </a>`)}),e.innerHTML=a}}function navigateTo(e,t=!0){currentPath=e||"/",t&&((pathHistory=pathHistory.slice(0,pathHistoryIndex+1)).push(currentPath),pathHistoryIndex++,e=buildUrlWithPath(currentPath),history.pushState({path:currentPath},"",e)),updateBreadcrumb(),loadFiles(currentPath),updateNavButtons()}function buildUrlWithPath(e){var t=new URL(window.location.href),i=document.getElementById("storageBadgeName")?.textContent;return i&&t.searchParams.set("storage",i),"/"!==e?t.searchParams.set("path",encodeURIComponent(e)):t.searchParams.delete("path"),t.toString()}function updateNavButtons(){var e=document.getElementById("backBtn"),t=document.getElementById("forwardBtn");e&&(e.disabled=pathHistoryIndex<=0),t&&(t.disabled=pathHistoryIndex>=pathHistory.length-1)}function goBack(){var e;0<pathHistoryIndex&&(pathHistoryIndex--,e=pathHistory[pathHistoryIndex],currentPath=e,history.back())}function goForward(){var e;pathHistoryIndex<pathHistory.length-1&&(pathHistoryIndex++,e=pathHistory[pathHistoryIndex],currentPath=e,history.forward())}function togglePathInput(){var e=document.getElementById("breadcrumb"),t=document.getElementById("pathInputWrapper"),i=document.getElementById("pathInput");"none"===e.style.display?(e.style.display="flex",t.style.display="none"):(e.style.display="none",t.style.display="flex",i.value=currentPath,i.focus(),i.select())}function confirmPathInput(){var e=document.getElementById("pathInput").value.trim();e&&navigateTo(normalizePath(e)),togglePathInput()}function cancelPathInput(){togglePathInput()}function normalizePath(e){var t,i=[];for(t of(e=e.startsWith("/")?e:"/"+e).split("/").filter(e=>e&&"."!==e))".."===t?0<i.length&&i.pop():i.push(t);return"/"+i.join("/")}function enterFolder(e){navigateTo(e)}function refresh(){loadFiles(currentPath)}function handleSearch(t){renderFiles(t?filesData.filter(e=>e.name.toLowerCase().includes(t.toLowerCase())):filesData)}async function previewFile(t,e){var i=e.split(".").pop().toLowerCase(),a=document.getElementById("previewContent"),n=document.getElementById("previewFileName"),n=(previewFilePath=t,previewZoom=1,previewRotation=0,previewPdfDoc=null,n&&(n.textContent=e),document.getElementById("zoomLevel"));n&&(n.textContent="100%");a.innerHTML='<div class="preview-placeholder"><i class="ti ti-loader"></i><p>加载中...</p></div>';let o;try{var r=await buildPublicRequest({path:t}),l=await(await fetch(API_BASE+"/download",{method:"POST",headers:{"Content-Type":"application/json"},body:JSON.stringify(r)})).json();o=200===l.code&&l.data&&l.data.download_url?l.data.download_url:API_BASE+"/fs/download?path="+encodeURIComponent(t)}catch(e){console.error("获取下载链接失败:",e),o=API_BASE+"/fs/download?path="+encodeURIComponent(t)}n=""+window.location.origin+o;if(["jpg","jpeg","png","gif","svg","webp","bmp","ico"].includes(i))previewFileType="image",a.innerHTML=`<img src="${n}" alt="${escapeHtml(e)}" onload="onImageLoad()" onerror="onPreviewError()">`;else if(["mp4","webm","ogg","mov","avi","mkv"].includes(i))previewFileType="video",a.innerHTML=`<video controls src="${n}"></video>`;else if(["mp3","wav","flac","aac","ogg"].includes(i))previewFileType="audio",a.innerHTML=`<audio controls src="${n}"></audio>`;else if(["txt","md","log","json","xml","yaml","yml","js","ts","py","java","c","cpp","go","rs","html","css","sh"].includes(i)){previewFileType="text";try{var s=await(await fetch(n)).text();a.innerHTML=`<pre>${escapeHtml(s.substring(0,1e5))}</pre>`}catch(e){a.innerHTML='<div class="preview-placeholder"><i class="ti ti-alert-circle"></i><p>无法加载文本内容</p></div>'}}else["pdf"].includes(i)?(previewFileType="pdf",await renderPdfPreview(n,a)):(previewFileType="other",a.innerHTML='<div class="preview-placeholder"><i class="ti ti-file"></i><p>此文件类型不支持预览</p><p style="margin-top:8px;color:var(--text-secondary)">您可以下载文件后查看</p></div>');document.getElementById("previewModal").style.display="flex"}async function renderPdfPreview(e,a){if("undefined"==typeof pdfjsLib&&(a.innerHTML=`
      <div class="pdf-loading">
        <div class="spinner"></div>
        <p class="loading-text">正在加载 PDF 组件...</p>
        <div class="progress-bar"><div class="progress" style="width: 30%"></div></div>
      </div>`,await waitForPdfJs(),"undefined"==typeof pdfjsLib))a.innerHTML=`<div class="preview-placeholder">
        <i class="ti ti-alert-circle"></i>
        <p>PDF 预览功能加载失败</p>
        <p style="margin-top:8px;color:var(--text-secondary)">请检查网络连接</p>
      </div>`;else{a.innerHTML=`
    <div class="pdf-loading">
      <div class="spinner"></div>
      <p class="loading-text">正在加载 PDF 文件...</p>
      <div class="progress-bar"><div class="progress" style="width: 0%"></div></div>
      <p class="loading-percent">0%</p>
    </div>`;try{var t=pdfjsLib.getDocument({url:e,cMapUrl:"https://cdnjs.cloudflare.com/ajax/libs/pdf.js/3.11.174/cmaps/",cMapPacked:!0}),i=(t.onProgress=function(e){var e=Math.round(e.loaded/e.total*100),t=a.querySelector(".progress"),i=a.querySelector(".loading-percent");t&&(t.style.width=e+"%"),i&&(i.textContent=e+"%")},previewPdfDoc=await t.promise,a.innerHTML='<div class="pdf-page-container" id="pdfPageContainer"></div>',document.getElementById("pdfPageContainer"));for(let e=1;e<=previewPdfDoc.numPages;e++){var n=document.createElement("div"),o=(n.className="pdf-page",n.style.marginBottom="16px",document.createElement("canvas")),r=(o.className="pdf-canvas",o.id="pdf-page-"+e,n.appendChild(o),i.appendChild(n),await previewPdfDoc.getPage(e)),l=r.getViewport({scale:previewZoom,rotation:previewRotation});o.height=l.height,o.width=l.width,await r.render({canvasContext:o.getContext("2d"),viewport:l}).promise}var s=document.createElement("div");s.style.color="var(--text-secondary)",s.style.fontSize="12px",s.style.marginTop="8px",s.textContent=`共 ${previewPdfDoc.numPages} 页`,i.appendChild(s)}catch(e){console.error("PDF 渲染失败:",e),a.innerHTML=`<div class="preview-placeholder">
      <i class="ti ti-alert-circle"></i>
      <p>PDF 加载失败</p>
      <p style="margin-top:8px;color:var(--text-secondary)">${e.message}</p>
    </div>`}}}function waitForPdfJs(){return new Promise(e=>{let t=0,i=setInterval(()=>{("undefined"!=typeof pdfjsLib||50<=++t)&&(clearInterval(i),e())},100)})}function onImageLoad(){}function onPreviewError(){document.getElementById("previewContent").innerHTML=`<div class="preview-placeholder">
    <i class="ti ti-alert-circle"></i>
    <p>加载失败</p>
  </div>`,showToast("预览加载失败","error")}function zoomIn(){var e;"pdf"===previewFileType?(previewZoom+=.25,updateZoomDisplay(),previewPdfDoc&&rerenderPdf()):"image"===previewFileType&&(previewZoom+=.25,updateZoomDisplay(),e=document.querySelector(".preview-content img"))&&(e.style.transform=`scale(${previewZoom})`)}function zoomOut(){var e;.5<previewZoom&&("pdf"===previewFileType?(previewZoom-=.25,updateZoomDisplay(),previewPdfDoc&&rerenderPdf()):"image"===previewFileType&&(previewZoom-=.25,updateZoomDisplay(),e=document.querySelector(".preview-content img"))&&(e.style.transform=`scale(${previewZoom})`))}function rotatePreview(){var e;previewRotation=(previewRotation+90)%360,"pdf"===previewFileType&&previewPdfDoc?rerenderPdf():"image"===previewFileType&&(e=document.querySelector(".preview-content img"))&&(e.style.transform=`rotate(${previewRotation}deg) scale(${previewZoom})`)}function updateZoomDisplay(){var e=document.getElementById("zoomLevel");e&&(e.textContent=Math.round(100*previewZoom)+"%")}async function rerenderPdf(){if(previewPdfDoc){var t=document.getElementById("pdfPageContainer");if(t){t.innerHTML='<div class="preview-placeholder"><i class="ti ti-loader"></i><p>渲染中...</p></div>';for(let e=1;e<=previewPdfDoc.numPages;e++){var i=document.createElement("div"),a=(i.className="pdf-page",i.style.marginBottom="16px",document.createElement("canvas")),i=(a.className="pdf-canvas",a.id="pdf-page-"+e,i.appendChild(a),t.appendChild(i),await previewPdfDoc.getPage(e)),n=i.getViewport({scale:previewZoom,rotation:previewRotation});a.height=n.height,a.width=n.width,await i.render({canvasContext:a.getContext("2d"),viewport:n}).promise}}}}function hidePreviewModal(){document.getElementById("previewModal").style.display="none",document.getElementById("previewContent").innerHTML="",previewPdfDoc=null,previewZoom=1,previewRotation=0}function downloadFromPreview(){previewFilePath&&downloadFile(previewFilePath)}async function downloadFile(t){try{var i,a=await buildPublicRequest({path:t}),n=await(await fetch(API_BASE+"/download",{method:"POST",headers:{"Content-Type":"application/json"},body:JSON.stringify(a)})).json();let e;200===n.code&&n.data&&n.data.download_url?(e=n.data.download_url,i=""+window.location.origin+e,window.open(i,"_blank"),showToast("下载已开始","success")):showToast("获取下载链接失败："+n.message,"error")}catch(e){console.error("下载失败:",e),showToast("下载失败："+e.message,"error")}}async function copyShareUrl(t){try{var i=await(await fetch(API_BASE+"/fs/get?path="+encodeURIComponent(t))).json();let e;200===i.code&&i.data&&i.data.url?(e=i.data.url).startsWith("http")||(e=""+window.location.origin+e):e=""+window.location.origin+API_BASE+"/fs/download?path="+encodeURIComponent(t),await copyToClipboard(e)?showToast("分享链接已复制到剪贴板","success"):showToast("复制失败","error")}catch(e){showToast("获取链接失败："+e.message,"error")}hideContextMenu()}function showContextMenu(e,t,i){e.preventDefault(),e.stopPropagation(),contextMenuTarget={path:t,type:i};var a=document.getElementById("contextMenu"),i="dir"===i?`
        <div class="context-menu-item" onclick="hideContextMenu(); enterFolder('${escapeHtml(t)}')">
            <i class="ti ti-folder-open"></i> 打开
        </div>
      `:`
        <div class="context-menu-item" onclick="hideContextMenu(); previewFile('${escapeHtml(t)}', '${escapeHtml(t.split("/").pop())}')">
            <i class="ti ti-eye"></i> 预览
        </div>
        <div class="context-menu-item" onclick="hideContextMenu(); downloadFile('${escapeHtml(t)}')">
            <i class="ti ti-download"></i> 下载
        </div>
        <div class="context-menu-item" onclick="hideContextMenu(); copyShareUrl('${escapeHtml(t)}')">
            <i class="ti ti-link"></i> 复制分享链接
        </div>
      `;a.innerHTML=`
    ${i}
    <div class="context-menu-divider"></div>
    <div class="context-menu-item" onclick="hideContextMenu(); copyPath('${escapeHtml(t)}')">
        <i class="ti ti-link"></i> 复制路径
    </div>
  `,a.style.display="block",a.style.left=e.clientX+"px",a.style.top=e.clientY+"px"}function showContextMenuForFile(e,t){let i=document.getElementById("contextMenu"),a=(i.style.display="none",contextMenuTarget={path:e,type:t},event.target.getBoundingClientRect());setTimeout(()=>{i.style.display="block",i.style.left=a.left+"px",i.style.top=a.bottom+8+"px"},0)}function hideContextMenu(){document.getElementById("contextMenu").style.display="none"}async function copyPath(e){await copyToClipboard(e)?showToast("路径已复制到剪贴板","success"):showToast("复制失败","error"),hideContextMenu()}document.addEventListener("DOMContentLoaded",()=>{var e=new URLSearchParams(window.location.search),t=e.get("storage"),e=e.get("path"),i=(t&&(currentPath=t,i=document.getElementById("storageBadge"),a=document.getElementById("storageBadgeName"),i)&&a&&(i.style.display="inline-flex",a.textContent=t),e&&(currentPath=decodeURIComponent(e)),pathHistory=[currentPath],window.addEventListener("error",e=>{console.error("全局错误:",e.error)}),window.addEventListener("unhandledrejection",e=>{console.error("未处理的 Promise rejection:",e.reason)}),pathHistoryIndex=0,initTheme(),loadFiles(),document.getElementById("searchInput")),a=(i&&i.addEventListener("input",e=>{handleSearch(e.target.value)}),document.addEventListener("click",e=>{e.target.closest(".context-menu")||hideContextMenu()}),window.addEventListener("popstate",e=>{e.state&&void 0!==e.state.path&&(currentPath=e.state.path,-1!==(e=pathHistory.indexOf(currentPath))&&(pathHistoryIndex=e),updateBreadcrumb(),loadFiles(currentPath),updateNavButtons())}),document.getElementById("pathInput"));a&&a.addEventListener("keypress",e=>{"Enter"===e.key&&confirmPathInput()})}),document.addEventListener("keydown",e=>{"Escape"===e.key&&(hidePreviewModal(),hideContextMenu())});