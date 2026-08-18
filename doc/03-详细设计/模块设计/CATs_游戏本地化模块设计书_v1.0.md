# CATs 游戏本地化模块设计书

**系统名称:** CATs — 全媒体 AI 辅助翻译 SaaS 平台

---

## 文档管理信息

| 项目 | 内容 |
|---|---|
| 文档编号 | CATs-DD-MOD-003 |
| 文档名 | 游戏本地化模块设计书（引擎适配 / 二进制资源对接 / 提取回写管线） |
| 版本 | 第 1.0 版（草稿） |
| 创建日 | 2026-08-18 |
| 作者 | 架构师 |
| 状态 | 评审前草稿 |
| 密级 | 仅社内 |
| 上游文档 | [CATs 微服务架构设计书 v1.0](../../02-基础设计/架构设计/CATs_微服务架构设计书_v1.0.md)、[CATs 模块设计书 v2.0](./CATs_模块设计书_v2.0.md)、[CATs 数据库设计书 v2.0](../数据库设计/CATs_数据库设计书_v2.0.md) |

### 修订履历

| 版本 | 日期 | 修订者 | 修订内容 |
|---|---|---|---|
| 1.0 | 2026-08-18 | 架构师 | 首版：游戏本地化领域需求拆解，Unity/Unreal/Godot 三引擎对接方案 |

---

## 0. 阅读指南

游戏本地化与文档/字幕本地化的本质差异在于：**待译文本不以"文件"为最小交付单元，而是深埋在引擎二进制资源、序列化对象、运行时字符串表中**，且往往存在标签/占位符/复数规则/UI 空间约束等强约束。本模块承接架构设计书"可插拔处理器"设计理念，新增一个媒体处理域：**游戏本地化域（game-localization）**，通过"引擎适配器（Engine Adapter）"模式解决 Unity / Unreal / Godot 三大引擎的对接问题，同时保持核心翻译管线（TM/术语/QA/AI 网关）完全复用、不因新增引擎而改动。

---

## 1. 问题域拆解

游戏本地化自动化要解决四类问题，本文档按此结构组织：

| 问题 | 说明 |
|---|---|
| ① 抽取（Extract） | 从引擎工程/构建产物中把待译字符串无损抽出为结构化数据，同时记录"如何写回"所需的全部上下文 |
| ② 保护（Protect） | 富文本标签、占位符、复数规则、性别变位等在翻译过程中必须原样保留或按目标语言规则重排 |
| ③ 回写（Inject） | 把翻译结果写回引擎能识别的格式，产出可直接被引擎加载的资源，不破坏工程结构 |
| ④ 验证（Verify） | UI 溢出/截断检测、字体缺字检测、伪本地化（pseudo-localization）、真机截图回归 |

---

## 2. 整体架构：引擎适配器模式

```
                    ┌─────────────────────────────────────────┐
                    │        game-localization-service          │
                    │  （新增媒体处理域微服务，遵循模块设计书§1.1  │
                    │    API/领域/仓储/基础设施四层分层）          │
                    └───────────────────┬─────────────────────┘
                                         │ domain::ports::EngineAdapter (trait/接口，依赖倒置)
        ┌────────────────────┬──────────┴───────────┬────────────────────┐
        ▼                    ▼                       ▼                    ▼
┌───────────────┐   ┌────────────────┐    ┌────────────────┐   ┌──────────────────┐
│ UnityAdapter   │   │ UnrealAdapter   │    │ GodotAdapter    │   │ （预留）CocosAdapter│
│ .asset/.meta/  │   │ .locres/.archive│    │ .po/.tres/.csv  │   │  ...              │
│ ScriptableObj  │   │ /UAsset/StringTbl│   │ gettext         │   │                   │
└───────────────┘   └────────────────┘    └────────────────┘   └──────────────────┘
        │                    │                       │
        └─────────────┬──────┴───────────────────────┘
                       ▼
        统一中间表示 GameLocaleUnit（领域模型，与引擎无关）
                       │
        ┌──────────────┼──────────────────┐
        ▼              ▼                  ▼
   TM 匹配复用    术语注入/标签保护     translation-core（gRPC 复用，
  （project-service）（tag_protector 扩展）  §4.3 已有能力，零改动）
                       │
                       ▼
              回写：对应 Adapter 的 inject()
                       │
                       ▼
         验证：pseudo-loc / 溢出检测 / 截图回归（game-qa 子模块）
```

**设计原则**：三引擎的差异被严格收敛在 `EngineAdapter` 实现内部（`extract()` / `inject()` / `protect_rules()` 三个接口方法），一旦字符串进入 `GameLocaleUnit` 统一模型，后续 TM/术语/AI 翻译/QA 全部复用架构设计书已有的 translation-core 管线，**不为游戏本地化单独建一条平行的翻译流水线**，避免维护两套 TM/QA 逻辑。

---

## 3. 统一中间表示：GameLocaleUnit

```protobuf
message GameLocaleUnit {
  string unit_id = 1;              // 引擎内唯一定位符，见下表
  string source_text = 2;
  string context_hint = 3;         // 变量名/对象路径/UI 组件名等上下文，供 AI 网关注入 prompt
  repeated Placeholder placeholders = 4;   // {0}/%s/{playerName} 等占位符位置与类型
  PluralSpec plural = 5;           // 复数规则（ICU MessageFormat / gettext plural forms）
  UiConstraint ui_constraint = 6;  // 显示区域像素宽高、最大字符数（引擎侧可选提供）
  string rich_text_dialect = 7;    // "unity_richtext" / "unreal_fstring" / "bbcode" / "none"
  map<string,string> engine_meta = 8;  // 各 Adapter 私有的回写所需元数据（原样透传，不做通用化）
}
```

`engine_meta` 是关键设计取舍：**不强行把三引擎的私有回写信息（如 Unity 的 `fileID`/`guid`，Unreal 的 `.locres` Namespace+Key，Godot 的 `.pot` msgctxt）提炼成统一字段**，而是作为不透明字段透传，只有产出方（Adapter）自己认识，避免"通用模型"被迫向最复杂引擎的结构看齐、拖累其余两个引擎的实现简洁性。

`unit_id` 编码规则（各引擎示例）：

| 引擎 | unit_id 格式 | 说明 |
|---|---|---|
| Unity | `unity://{guid}#{fileID}` | `guid` 对应 `.meta` 文件的 GUID，`fileID` 定位序列化对象内的具体字段 |
| Unreal | `unreal://{Namespace}/{Key}` | 对应 `.locres`/`Localization Dashboard` 的 Namespace+Key 组合，Unreal 官方本身即以此为本地化唯一标识 |
| Godot | `godot://{pot_context}/{msgid_hash}` | 对应 gettext 的 msgctxt，多语言场景下 `msgid` 可能重复，需 context 消歧 |

---

## 4. Unity 适配器设计

### 4.1 待译文本来源与抽取策略

Unity 项目中字符串分散在多处，需分策略处理，不能只做"扫 .cs 源码里的字面量"：

| 来源 | 抽取方式 |
|---|---|
| Unity 官方 Localization Package（`com.unity.localization`）的 String Table Collection | **首选路径**：直接解析 `.asset` 序列化的 `StringTableCollection`/`StringTable`，Unity YAML 文本序列化模式（Project Settings 设为 Force Text）下可直接文本解析，二进制模式（Force Binary）需走 Unity Editor 侧脚本导出 |
| 自定义 ScriptableObject（很多团队不用官方 Localization Package，自建对话/文案表） | 需项目方提供字段映射配置（哪个 Class 的哪些 `string` 字段是待译文本），无法通用猜测——纳入"项目本地化配置文件"（§7）声明 |
| UI 组件（`TextMeshPro`/`Text` 组件挂在 Prefab/Scene 上的硬编码文案） | 扫描 `.prefab`/`.scene` YAML，定位 `TMP_Text`/`Text` 组件的 `m_text` 字段；**明确标记为"反模式"在产出报告中提示项目方**，硬编码 UI 文案应迁移至 String Table，抽取仅做只读发现+报告，不做自动回写（回写 Prefab 风险高，容易破坏 Prefab 覆盖链） |
| C# 源码内联字符串（对话脚本用字面量而非表驱动） | 静态扫描 + 启发式规则（排除日志/调试字符串，通过 Attribute 标注 `[Localizable]` 或正则白名单），**准确率有限，作为"待人工确认候选清单"而非自动纳入翻译**，不承诺 100% 召回 |

### 4.2 二进制/序列化格式处理

Unity 的 `.asset`/`.prefab`/`.scene` 文件有两种序列化模式，处理方式完全不同：

- **YAML 文本模式（推荐项目方开启，Editor → Project Settings → Editor → Asset Serialization = Force Text）**：可用纯文本方式解析（`unity_yaml_parser` 内部模块，处理 Unity YAML 的多文档 `---` 分隔与 `!u!` 类型标签），无需启动 Unity Editor 进程，抽取/回写均可在无 Unity 环境的 CI 容器中完成，是**首选支持路径**。
- **二进制模式**：无法在 CI 侧脱离 Unity 解析，需依赖 §6 描述的 Editor 插件方式，由挂在项目里的 Editor 脚本在 Unity 进程内导出 JSON 中间文件后再交给 game-localization-service 处理，**明确告知客户"二进制序列化模式下无法做到全自动 CI 管线，需保留 Editor 插件人工/半自动触发环节"**，不过度承诺纯服务端能力。

### 4.3 AssetBundle / Addressables 场景

若项目已构建为 AssetBundle，**不支持对构建产物做反编译式抽取**（技术上可行但极脆弱、易随 Unity 版本升级失效，且多数团队工程源码可得，无需碰构建产物）。仅支持源工程级抽取；仅当客户明确要求"仅有构建产物、无源工程"这一边缘场景时，作为付费定制评估，不纳入标准功能范围。

### 4.4 回写与 Rich Text 保护

- 回写目标：将翻译结果写回 String Table `.asset`（YAML 文本模式）对应 Entry 的 `m_Localized` 字段，保持文件内其余条目字节级不变（仅做定向字符串替换，不做整体重新序列化，避免 Unity 内部字段顺序/GUID 因重新写出而产生无关 diff，方便美术/程序侧 code review 时能一眼看出"这次改动只涉及文本"）。
- Rich Text 标签保护：`<b>`/`<color=#RRGGBB>`/`<sprite name=...>` 等 TextMeshPro/UGUI 富文本标签，在 §4.3 `tag_protector` 模块中新增 `unity_richtext` 方言规则（复用现有标签保护框架，仅扩展一套正则/AST 规则，不新建独立保护模块）。

---

## 5. Unreal 适配器设计

### 5.1 官方本地化管线对齐

Unreal 自带完整的本地化系统（Localization Dashboard + Gather/Compile 流程），**本模块不重新发明一套抽取逻辑，而是对接 Unreal 官方产物**：

1. 项目方在 Unreal Editor 内执行标准 **Gather Text**（或 CI 中用 `UnrealEditor-Cmd.exe -run=GatherText -config=Config/Localization/Game_Gather.ini`），产出 `.archive`（原文+已有译文）与 `.manifest`（源文本清单，含 Namespace/Key/SourceLocation 上下文）文本文件（均为可读的 INI-like/JSON 格式，非二进制，天然适合服务端解析）。
2. game-localization-service 的 `UnrealAdapter.extract()` 直接解析 `.manifest`+`.archive`，产出 `GameLocaleUnit`；`FText` 的 `SourceLocation`（源码文件+行号）作为 `context_hint` 注入，帮助 AI 翻译理解文本出现场景（如"这是任务名"还是"这是道具描述"）。
3. 翻译完成后写回 `.archive` 文件对应 Key 的 `Translation` 字段。
4. **回写后仍需项目方在 Unreal Editor / CI 内执行标准 **Compile Text**（`-run=GatherText -config=Config/Localization/Game_Compile.ini`）**将 `.archive` 编译为运行时加载的 `.locres` 二进制文件——**这一步明确不由 game-localization-service 代劳**，因为 `.locres` 编译依赖 Unreal Editor 自身（需要引擎版本环境），本模块只负责产出标准 `.archive`，编译步骤纳入客户 CI 流水线（提供集成文档/CI 脚本示例，而非把 Unreal Editor 打进本服务的运行环境——避免为对接一个引擎把服务镜像做成臃肿的"塞了半个 Unreal Editor"的怪异形态）。

### 5.2 UAsset 内嵌字符串（DataTable/StringTable Asset）

部分团队不用 Gather 而是把文案放在 `DataTable`（`.uasset`，UE 序列化二进制）里。UAsset 是 Unreal 引擎版本强绑定的私有二进制格式，社区有 `UAssetAPI`（C#）可解析常见资产类型但对自定义 `UStruct`/引擎版本变化脆弱。方案：

- **标准路径**：引导客户将 `DataTable` 中的文案列改用 `FText` 并纳入 Gather 流程（Unreal 官方推荐做法），本模块提供检测脚本扫描项目中"文案列类型为 `FString` 而非 `FText`"的 DataTable 并生成迁移建议报告。
- **兼容路径**：若客户 DataTable 结构固定且愿意提供 `Row Struct` 的字段映射配置，`UnrealAdapter` 提供基于 `UAssetAPI`（作为可插拔子适配器，隔离在独立 sidecar 进程/容器内运行，避免其对 UE 版本的强假设污染主服务稳定性）的 CSV 导入导出通道；**明确作为"有限支持"能力**，不同 UE 版本/自定义 UStruct 兼容性需项目侧验证，不做通用保证。

### 5.3 占位符与复数规则

`FText::Format` 使用 `{PlayerName}`/`{ItemCount}|plural(...)` 语法（Unreal 内建 ICU-like 复数/性别语法），`UnrealAdapter.protect_rules()` 解析这套语法产出 `PluralSpec`/`Placeholder`，与 §7 描述的复数规则统一处理框架对接（同一套逻辑也服务 Godot 的 gettext 复数规则、Unity 若使用 Smart Format 的场景），**复数/占位符解析器按"语法方言"参数化实现，三引擎共享同一套核心状态机，只是词法规则表不同**，避免三份平行实现。

---

## 6. Godot 适配器设计

Godot 是三者中**对接成本最低**的引擎，因其官方本地化方案直接基于标准 gettext（`.po`/`.pot`）与简单的 CSV 翻译表，均为纯文本格式：

| 来源 | 处理方式 |
|---|---|
| gettext（`.po`/`.pot`），Godot 4.x 推荐方式 | 用标准 `polib`（已是 Python 生态成熟库，直接复用，不自研 PO 解析器）解析，`msgctxt` 映射 `unit_id`，`msgid_plural`/`msgstr[n]` 原生对应 `PluralSpec`，是三引擎中复数规则处理**最规整**的一条路径 |
| CSV 翻译表（Godot legacy 方式，`.csv`，首列 key，后续列各语言） | 直接按行解析，回写为新增/更新目标语言列，因是纯表格结构，甚至可直接复用架构设计书已有的通用表格类文件处理逻辑，不需要游戏专属解析代码 |
| `.tscn`/`.tres` 场景/资源文件内硬编码文案 | 与 Unity Prefab 同理，Godot 场景文件也是文本格式（`.tscn` 默认即文本），可解析但同样**只做只读发现报告**，建议迁移至 `tr()`/gettext 标记文本，不做场景文件自动回写（避免破坏场景节点树结构的风险） |

Godot 无需单独的"编译"步骤（`.po` 可直接被 Godot 运行时加载，或项目方选择转 `.mo` 提升加载性能，属可选优化非必须环节），三引擎中**唯一可以做到"抽取→翻译→回写"全自动闭环、无需引擎 Editor 环境参与**的路径，适合作为客户试点优先推荐的引擎。

---

## 7. 项目本地化配置文件（跨引擎统一约定）

三引擎抽取范围存在天然的"无法通用猜测"部分（自定义 ScriptableObject 字段、DataTable 行结构映射等），统一通过项目根目录 `cats-gameloc.yaml` 声明，避免每次都靠人工在控制台里点选配置：

```yaml
engine: unity              # unity | unreal | godot
version_hint: "2022.3 LTS" # 供 Adapter 选择兼容的序列化规则分支（如 Unity 不同版本 YAML 字段名差异）
serialization: text        # unity 专用：text | binary（§4.2）
extract:
  string_tables: true                     # 官方 Localization Package / Gather 产物，默认开启
  scan_prefabs_readonly: true             # UI 硬编码扫描，只读报告
  scan_source_literals: false             # 源码字面量启发式扫描，默认关闭（噪音大，按需开启）
  custom_scriptable_objects:              # Unity 专属：自定义表驱动文案类的字段映射
    - class_name: "DialogueEntry"
      text_fields: ["chineseText"]
      context_field: "speakerName"
placeholder_dialect: icu    # icu | printf | unity_smartformat | unreal_ftext
ui_constraint_source: none  # none | figma_export | engine_reported（若引擎侧能上报组件像素尺寸）
```

该文件纳入客户代码仓库版本管理（与 `.gitlab-ci.yml`/`.github/workflows` 同级），CI 触发抽取时读取，保证"抽取范围"这一决策可评审、可追溯，而非隐藏在 SaaS 控制台的某个开关里（架构设计书强调的配置即代码原则延伸到本模块）。

---

## 8. 引擎侧集成方式：Editor 插件 vs CI 离线管线

两种触发模式并存，按引擎/客户环境能力选择，不强制二选一：

| 模式 | 适用场景 | 实现方式 |
|---|---|---|
| **CI 离线管线**（首选，三引擎均支持文本格式路径时） | 客户已有 CI（Jenkins/GitLab CI/GitHub Actions），希望"提交代码→自动抽取→机翻+TM 预填→生成待审校任务→回写 PR" | 提供 `cats-gameloc-cli`（跨平台命令行工具，Rust 实现，静态链接免安装依赖，复用 `cats-sdk-rs`），CI 脚本调用 `cats-gameloc-cli extract` / `cats-gameloc-cli inject`，通过 REST API 与 game-localization-service 通信；产出以 Git Diff/PR 形式回传，纳入客户既有代码评审流程，而非绕开代码评审直接写主干 |
| **Editor 插件**（Unity 二进制序列化模式 / Unreal DataTable 场景 / 需要美术人员在编辑器内实时预览译文的场景） | 需要在引擎进程内执行导出（如 Unity 二进制模式）；或希望策划/UI 在 Editor 内直接看到"译文替换后的实际显示效果"辅助判断是否溢出 | Unity：`com.cats.localization-plugin` UPM 包，提供 Editor Window 触发导出/导入 + 场景内 In-Context 预览（临时切换语言查看 UI）；Unreal：Editor Utility Widget + Python（`unreal` 模块）脚本，封装 GatherText/CompileText 调用并触发与后端的同步；Godot：GDScript/C# 编写的 EditorPlugin，因 Godot 格式本身文本化，插件主要承担"一键触发 CLI + 结果导入编辑器内预览"的便捷入口角色，而非必需路径 |

Editor 插件与 CLI 共享同一套后端 API 契约（复用架构设计书 §21 的接口设计规范），**插件本身是瘦客户端，不重复实现抽取/回写逻辑**——避免出现"Editor 插件里一套解析代码、CI CLI 里另一套"的双份维护成本；实际的 Adapter 解析逻辑始终在 game-localization-service 侧，插件/CLI 均通过上传原始工程文件（或客户端本地解析后仅上传 `GameLocaleUnit` JSON，视文件是否允许离开客户网络环境而定，见 §10 私有化部署考虑）与服务端交互。

---

## 9. UI 约束与验证（游戏本地化特有 QA 环节）

游戏本地化的 QA 环节比常规文档翻译多出"空间约束"这一维度，纳入独立子模块 `game-qa`：

| 检查项 | 方式 |
|---|---|
| 伪本地化（Pseudo-localization） | 抽取阶段可选生成伪译文（字符加长 30%~50%、注入重音字符、加方括号标记边界），不经真实翻译即可提前在引擎内跑一遍 UI，暴露"未做本地化适配的写死宽度控件"，这是本模块**优先级最高的低成本高收益能力**，建议作为客户试点第一个演示功能 |
| 字符宽度估算溢出检测 | 若 `GameLocaleUnit.ui_constraint` 提供了像素宽高（来自 Unity `RectTransform`/Unreal `UMG Slot`/Godot `Control` 节点尺寸，由 Editor 插件在场景内读取上报），结合目标语言字体的字符宽度表（CJK/拉丁字符宽度差异显著）做初步估算，超阈值标记"疑似溢出"供审校重点关注；**明确这是启发式估算不是精确渲染结果**，非最终裁决 |
| 真机/引擎截图回归 | 更可靠但成本更高的方式：Editor 插件在切换语言后对指定场景/UI 截图，与基线截图做像素/结构差异对比（复用图像 diff 通用能力），标记显著差异区域；此环节依赖引擎进程实际渲染，**不在 game-localization-service 内实现，而是作为 Editor 插件本地能力**，截图对比结果作为附件上传供审校查看 |
| 字体缺字检测 | 目标语言（尤其新增 CJK/阿拉伯语/泰语等客户原本未覆盖的语言）字符集是否被项目当前字体资源覆盖，缺字会在引擎内显示为方块/问号；抽取阶段提取译文全字符集，与客户提供的字体文件（TTF/OTF 直接读 cmap 表，跨引擎通用逻辑）做差集比对，产出"缺字清单"报告 |

---

## 10. 数据模型与私有化部署考虑

### 10.1 新增数据表（挂载于既有 project_db，遵循数据库设计书既有规范，不新建独立数据库）

```sql
CREATE TABLE game_locale_units (
    id              BIGSERIAL PRIMARY KEY,
    project_id      UUID NOT NULL REFERENCES projects(id),
    engine          TEXT NOT NULL CHECK (engine IN ('unity','unreal','godot')),
    unit_id         TEXT NOT NULL,        -- §3 引擎专属格式，如 unity://{guid}#{fileID}
    source_text     TEXT NOT NULL,
    context_hint    TEXT,
    placeholders_json  JSONB,
    plural_json     JSONB,
    ui_constraint_json JSONB,
    rich_text_dialect  TEXT,
    engine_meta_json   JSONB NOT NULL,    -- 不透明透传字段，见 §3
    content_hash    TEXT NOT NULL,        -- source_text 归一化后哈希，驱动增量抽取（§10.2）
    UNIQUE (project_id, engine, unit_id)
);
CREATE INDEX idx_glu_project_hash ON game_locale_units(project_id, content_hash);
```

### 10.2 增量抽取（避免每次全量重译）

每次 CI 抽取时以 `content_hash`（`source_text` 归一化后哈希）比对上次快照：仅 `content_hash` 变化或新增的 `unit_id` 进入本次翻译任务，未变化的沿用既有 TM 精确匹配结果直接复用（本质是 TM 100% 匹配的特例，复用 translation-core 既有 TM 匹配逻辑而非新造一套 diff 机制）。已翻译但源文本已被删除的 `unit_id` 标记 `orphaned` 而非立即物理删除，保留一个宽限期供人工确认（防止因抽取范围配置临时改动误判导致历史译文丢失）。

### 10.3 私有化部署考虑

游戏客户对源码/工程资产的保密要求通常高于文档翻译客户（未发布游戏的剧情文本/美术资源泄露风险敏感），本模块需与架构设计书已有的私有化部署方案（若有）对齐，明确两种数据流模式供客户选择：

- **云端处理模式**：客户 CI 直接上传工程文件片段（仅涉及文案的 `.asset`/`.archive`/`.po` 等，不含贴图/模型等大体积美术资源）至 game-localization-service。
- **本地解析模式**：Editor 插件/CLI 在客户内网本地完成 Adapter 的 `extract()` 解析，仅将脱离了工程上下文的 `GameLocaleUnit` 结构化 JSON（不含原始工程文件）上传云端做翻译，回写产物同样在本地由插件/CLI 完成——**这是对游戏客户的关键差异化能力**，需在 Adapter 接口设计上从一开始就保证"解析逻辑可运行在客户本地"（即 Adapter 实现本身不依赖必须访问 game-localization-service 内部私有服务，可编译为独立二进制分发），而非默认假设所有解析都在云端完成。

---

## 11. 分阶段落地建议

不建议三引擎同时启动，按"投入产出比"排序：

| 阶段 | 范围 | 理由 |
|---|---|---|
| Phase 1 | Godot（gettext/CSV） + Unity（YAML 文本模式 String Table） | 均为纯文本格式，无需引擎 Editor 环境参与，服务端即可完成抽取/回写闭环，验证核心架构（EngineAdapter 接口/GameLocaleUnit 模型/增量抽取）投入最小 |
| Phase 2 | Unreal（Gather/Compile 对接） | 依赖客户自行完成 Compile 步骤，需补充 CI 集成文档与脚本模板，工作量集中在文档/客户支持而非核心解析代码 |
| Phase 3 | Unity 二进制模式 Editor 插件、Unreal UAsset/DataTable 兼容路径、game-qa 截图回归 | 依赖引擎进程内插件开发，跨平台/跨引擎版本兼容性验证成本最高，作为增值能力后置 |

---

## 12. 与既有架构的接口影响面（供架构评审确认）

- 新增微服务 `game-localization-service`，遵循模块设计书 §1.1 分层规范，`domain::ports::EngineAdapter` 三个实现（Unity/Unreal/Godot）放在 `infra` 层。
- `translation-core`（§4.3）**零改动**复用，`GameLocaleUnit` 转换为其既有 `TranslateBatch` 输入分段格式即可。
- `tag_protector`（既有标签保护模块）扩展新增 `unity_richtext`/`unreal_ftext`/`bbcode` 三种方言规则，属于扩展现有模块而非新建。
- 数据库新增 `game_locale_units` 表挂载于既有 `project_db`，不新建数据库实例。
- 新增交付物 `cats-gameloc-cli`（Rust，复用 `cats-sdk-rs`）与三个引擎 Editor 插件（Unity UPM 包 / Unreal Editor Utility+Python / Godot EditorPlugin），均为独立分发物，不并入 Tauri 客户端主程序。
