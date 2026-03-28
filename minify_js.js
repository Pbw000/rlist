const fs = require("fs");
const path = require("path");
const { execSync } = require("child_process");

const srcDir = path.join(__dirname, "static_src");
const destDir = path.join(__dirname, "static");

// 创建目标目录
if (!fs.existsSync(destDir)) {
  fs.mkdirSync(destDir, { recursive: true });
}

// 复制 HTML 文件（不压缩）
const htmlFiles = ["admin.html", "index.html", "public.html"];
htmlFiles.forEach((file) => {
  const srcPath = path.join(srcDir, file);
  const destPath = path.join(destDir, file);
  if (fs.existsSync(srcPath)) {
    fs.copyFileSync(srcPath, destPath);
    console.log(`Copied: ${file}`);
  }
});

// 复制 CSS 目录
const cssSrcDir = path.join(srcDir, "css");
const cssDestDir = path.join(destDir, "css");
if (fs.existsSync(cssSrcDir)) {
  fs.mkdirSync(cssDestDir, { recursive: true });
  fs.readdirSync(cssSrcDir).forEach((file) => {
    const srcPath = path.join(cssSrcDir, file);
    const destPath = path.join(cssDestDir, file);
    fs.copyFileSync(srcPath, destPath);
    console.log(`Copied CSS: ${file}`);
  });
}

// 复制 fonts 目录
const fontsSrcDir = path.join(srcDir, "fonts");
const fontsDestDir = path.join(destDir, "fonts");
if (fs.existsSync(fontsSrcDir)) {
  fs.mkdirSync(fontsDestDir, { recursive: true });
  fs.readdirSync(fontsSrcDir).forEach((file) => {
    const srcPath = path.join(fontsSrcDir, file);
    const destPath = path.join(fontsDestDir, file);
    fs.copyFileSync(srcPath, destPath);
    console.log(`Copied Font: ${file}`);
  });
}

// 创建 js 目录并压缩 JS 文件
const jsSrcDir = path.join(srcDir, "js");
const jsDestDir = path.join(destDir, "js");

if (fs.existsSync(jsSrcDir)) {
  fs.mkdirSync(jsDestDir, { recursive: true });

  const jsFiles = fs.readdirSync(jsSrcDir).filter((f) => f.endsWith(".js"));

  jsFiles.forEach((file) => {
    const srcPath = path.join(jsSrcDir, file);
    const destPath = path.join(jsDestDir, file);

    try {
      // 安全混淆配置：只混淆局部变量，保留顶层函数名
      // -c: 压缩代码
      // -m: 混淆变量名（不混淆顶层和函数名）
      execSync(`uglifyjs "${srcPath}" -o "${destPath}" -c -m`, {
        stdio: "inherit",
      });

      console.log(`Minified: ${file}`);
    } catch (error) {
      console.error(`Failed to minify ${file}: ${error.message}`);
      // 如果压缩失败，直接复制原文件
      fs.copyFileSync(srcPath, destPath);
      console.log(`Copied (no minification): ${file}`);
    }
  });
}

console.log("\nDone! Output directory:", destDir);
