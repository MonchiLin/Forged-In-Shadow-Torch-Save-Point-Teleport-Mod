# 图像展示前端项目

基于 Vite 的桌面演示界面，用于浏览传送控制台草图。顶部切换 6 个区域，中央面板显示放大的地图，支持缩放、拖拽和锚点复制。

## 地图配置

所有地图元数据位于 src/config/maps.json：
- image：对应的 PNG 资源路径
- markers：标记相对坐标（百分比）与复制文本
- 	heme：可选的额外样式变量（当前仅保留 --map-image-filter）

修改 JSON 后重新运行开发或构建命令即可生效。

## 使用方法

`powershell
npm install
npm run dev
`

根据终端输出的地址打开浏览器。滚轮缩放（双击复位），按住左键拖动画面，点击标记即可复制锚点编号。若需生成静态资源，执行 
pm run build，构建结果位于 dist/。
