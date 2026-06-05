# 分类图标目录（web-starter）

- **源码**：`src/icons/catalog.ts`（19 套主题 × 语义 → 不同 Lucide 图标 + 描边/颜色）
- **渲染**：`CategoryIcon` 读 `<html data-theme>`，切换主题时图标自动换风格
- **数据**：`src/data/links.ts` 里每个分类写 `semantic`（如 `fashion`），不要写 `emoji`
- **覆盖**：分类可写显式 `icon: { name: "Watch" }`；后续由工厂 `PATCH .../site-config` 写入

## 扩展新主题

1. 在 `src/themes/registry.ts` 增加 `ThemeMeta`
2. 在 `catalog.ts` 的 `THEME_ICON_STYLES` 与 `THEME_ICON_OVERRIDES` 各加一行

## 扩展新语义

在 `SEMANTIC_ICONS` 增加键，并在 `resolve.ts` 的 `ID_ALIASES` / `NAME_PATTERNS` 补中文/英文别名。
