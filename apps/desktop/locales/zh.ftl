# ATENA – 界面文本（简体中文）。
# 参考语言为斯洛伐克语（sk.ftl）；此处每条消息的标识符与之相同。
# 机器翻译：在母语者审校之前为草稿。

language-name = 中文（简体）

## 应用

app-claim = 经过核实的教育信息
app-version = 版本 { $version } · 开源软件
translation-draft = 翻译 – 草稿
yes = 是
no = 否
action-close = 关闭
action-cancel = 取消
action-open-original = 打开原文

## 启动

startup-starting = ATENA 正在启动…
startup-first-time = 首次启动时，ATENA 会在这台电脑上建立自己的数据库并载入来源登记表。大约需要半分钟；下次会更快。
startup-failed-hint = 请关闭 ATENA 后重新打开。如果错误再次出现，详细信息见应用文件夹中的日志（logs/atena.log）。

## 导航

nav-label = 主菜单
nav-overview = 概览
nav-search = 搜索
nav-tools = 工具
nav-sources = 来源
nav-proposals = 建议
nav-crawl = 采集
nav-settings = 设置

## 概览

overview-title = 概览
overview-empty-title = 尚未采集任何内容
overview-empty-text = 请开始采集。ATENA 以有礼貌的访客身份访问登记表中的来源：遵守其规则，从不急于求成，只取法律和许可允许的内容。
overview-empty-action = 前往采集
overview-documents = 份文档
overview-versions = { $count } 个版本
overview-searchable = 份可搜索的文档
overview-sources = 个来源正在采集（登记表中的全部来源）
overview-proposals = 条建议等待决定
overview-last-check = 最近一次检查：{ $time } · 过去 24 小时内共 { $checks } 次检查
overview-never-checked = 尚未检查过任何来源。
overview-newest = 最新
overview-search-all = 搜索全部
overview-sections = 类别
overview-in-scope = 当前范围共 { $total } 篇文档
overview-new-in-scope = 近 { $days } 天新增 { $new } 篇
overview-counts-legend = 类别旁的数字：近 { $days } 天新增 / 全部
overview-counts-title = 近 { $days } 天新增 { $new } 篇，共 { $total } 篇
overview-sources-line = 正在采集 { $sources } 个来源中的 { $crawled } 个

## 斯洛伐克或国外数据，来源可信度

scope-label = 显示哪些数据
scope-sk = 斯洛伐克
scope-abroad = 国外
scope-languages = 国外数据的语言
scope-all-languages = 所有语言
trust-official = 官方来源
trust-expert = 专业来源
trust-other = 其他来源——请核实
tools-title = 工具与词典
tools-intro = ATENA 不采集的词典、语料库和应用——请直接在其中检索。它们会在您的浏览器中打开。
tools-open = 打开工具
tools-none = 这些数据暂时没有工具。

## 搜索

search-title = 搜索
search-placeholder = 方法、法规、资料……任何语言均可
search-action = 搜索
search-hint = 用任何语言输入您要查找的内容。ATENA 会通过识别出的概念在其他语言中一并搜索。
search-hint-syntax = 短语请加引号（“阅读素养”），用减号排除某个词（-假期）。
search-filters = 已筛选：
search-filter-remove = 取消此筛选
search-narrow = 缩小结果范围
search-found =
    { $count ->
        [0] 未找到任何文档。
       *[other] 找到 { $count } 份文档。
    }
search-some-words = 没有文档包含全部词语——以下文档包含其中一部分。
search-concepts = 同时搜索了这些概念：
search-nothing = 请尝试其他词语或取消部分筛选。
search-previous = 上一页
search-next = 下一页
search-page = 第 { $page } 页，共 { $pages } 页
search-only-official = 仅官方来源
search-tools = 与搜索词相关的工具
hit-no-text = 此来源的文本不在 ATENA 中显示——其许可不允许。请打开原文。

facet-section = 栏目
facet-school-type = 学校类型
facet-resource-type = 内容类型
facet-topic = 主题
facet-country = 国家
facet-authority = 来源权威性
facet-language = 语言
facet-source = 来源
facet-trust = 来源可信度

## 文档卡片

document-title = 文档
document-untitled = （无标题）
document-not-shown = ATENA 不显示此文档：它已被删除，其发布者不希望被搜索到，或其来源不得采集。
document-published = 发布于 { $date }
document-labels-are-suggestions = 栏目和主题的归类是 ATENA 的建议，需经人工核实。
document-text = 正文
document-text-not-shown = ATENA 不显示全文——来源的许可（{ $licence }）不允许。请阅读原文。
document-versions = { $count } 个版本
version-number = 版本
version-captured = 采集时间
version-change = 变更
version-paragraphs = （+{ $added } / −{ $removed } 段）
version-numbers-changed = 数字或日期有变

change-initial = 首次采集
change-minor = 细微变更
change-substantive = 实质性变更
change-changed = 文本已变更（未保留先前版本）

## 来源

sources-title = 来源
sources-intro = 登记表中的来源和已确认的建议。每个来源都显示合法性原则允许什么：能否采集、能否保存文本、能否完整显示。
sources-sync = 同步共享登记表
sources-sync-explain = ATENA 将从 GitHub 上的 ATENA 仓库下载最新的共享来源登记表（文件 data/sources.yaml，仅通过加密的 https 连接），并据此更新数据库。合法性原则不再允许的内容将被删除。
sources-sync-privacy = 只发送一个普通的文件请求；不会发送任何关于您或您的建议的信息。
sources-sync-confirm = 下载并同步
sources-synced = 登记表已同步：{ $sources } 个来源，{ $added } 个新地址。删除的文档：{ $purged }，仅保留元数据的版本：{ $restricted }。
sources-show-all = 全部
sources-show-crawled = 正在采集
sources-show-not-crawled = 不采集
sources-show-imported = 来自建议
sources-filter = 按名称、地址或国家筛选
sources-column-source = 来源
sources-column-country = 国家
sources-column-rights = 许可与条款
sources-column-crawl = 采集
sources-column-text = 文本
sources-column-show = 显示
sources-column-documents = 文档
sources-checked = 检查于 { $date }
sources-tool = 工具（不采集）
sources-site-wide = 整个网站
sources-none = 没有符合条件的来源。

allowed-crawl = 可以采集
allowed-text = 可以保存文本
allowed-show = 可以显示全文

licence-copyrighted = 受版权保护
licence-official = 官方文本（不受版权保护）

terms-allowed = 条款允许
terms-forbidden = 条款禁止
terms-reserved = 已保留（文本挖掘）
terms-unknown = 条款未核实

state-proposed = 建议
state-confirmed = 已确认
state-rejected = 已拒绝
state-merged = 已并入登记表

restriction-no-profile = ATENA 没有该来源所在国家的法律档案
restriction-terms-forbid = 使用条款禁止自动访问
restriction-terms-not-verified = 受保护内容：使用条款尚未经人核实
restriction-research-only = 受保护内容：来源国家仅允许为非商业研究进行挖掘
restriction-no-mining-exception = 受保护内容：来源国家没有文本与数据挖掘的例外规定
restriction-mining-reserved = 发布者保留了文本与数据挖掘的权利
restriction-privacy-law = 来源国家的数据保护法不允许在境外保存文本

## 建议

proposals-title = 建议的来源
proposals-intro = 建议 ATENA 应关注的网页。它会先评估每个地址；只有在有人确认建议后才开始采集。
proposals-tab-add = 添加地址
proposals-tab-waiting = 等待决定
proposals-tab-decided = 已决定
proposals-tab-share = 交给编辑
proposals-tab-receive = 接收列表

import-addresses = 地址或 DOI，每行一个
import-placeholder = https://www.school.cn/news/
import-format = 可在地址后加上适用其法律的国家代码（例如“https://www.skola.cz/ CZ”）。每次最多 500 个地址。
import-country = 无代码地址所属国家
import-country-auto = 根据地址判断
import-how = ATENA 以有礼貌的访客身份打开每个地址：读取 robots.txt、许可声明和使用条款，并提出法律允许的做法。在您确认之前不会采集任何内容。关于个人的网页即使作为建议也不会保存。
import-action =
    { $count ->
        [0] 评估
       *[other] 评估 { $count } 个地址
    }
import-busy = 正在评估……地址较多时可能需要几分钟。
import-done = 已评估 { $count } 个地址。
import-kept-for-sharing = { $count } 个地址等待交给编辑。
import-column-line = 行
import-column-address = 地址
import-column-result = 结果
import-column-proposal = 建议

outcome-new = 新建议
outcome-reassessed = 建议已重新评估
outcome-already-proposed = 已被建议
outcome-already-confirmed = 已被确认
outcome-already-rejected = 已被拒绝
outcome-already-merged = 已在登记表中
outcome-in-register = 已在登记表中（{ $register }）
outcome-blocked = 因删除请求而被屏蔽
outcome-personal-page = 关于个人的网页——不保存

problem-robots = robots.txt 不允许 ATENA 读取该网页
problem-robots-unreachable = 无法读取 robots.txt，因此暂不读取任何内容
problem-blocked = 不允许访问该地址（私有网络、非常规端口……）
problem-status = 服务器返回了错误
problem-unreachable = 无法连接服务器
problem-retry-later = 服务器要求稍后再来
problem-unsupported-type = ATENA 不读取此类内容
problem-too-large = 内容超出 ATENA 读取的大小
problem-dangerous = 它提供程序或含有活动内容的文档——ATENA 不使用此类来源
problem-too-demanding = 该内容所需的内存或时间超过任何真实文档——ATENA 不使用它

reason-cannot-read = 无法读取该地址
reason-personal-page = 关于个人的网页
reason-automation-ban = 条款禁止自动访问
reason-personal-use-only = 条款仅允许个人使用
reason-mining-reserved = 条款保留了文本挖掘或 AI 使用
reason-tdm-reserved = TDMRep 保留了文本与数据挖掘
reason-ai-crawlers-refused = robots.txt 拒绝 AI 采集程序
reason-open-licence = 开放许可
reason-licence-not-open = 许可不开放或自相矛盾
reason-non-commercial = 仅限非商业使用
reason-rights-reserved = 保留所有权利
reason-terms-read = 已读取条款
reason-no-terms-found = 未找到条款
reason-terms-unreadable = 无法读取条款
reason-unknown-country = 需要指定国家
reason-dangerous-content = 危险内容

rejection-not-an-address = 既不是地址也不是 DOI
rejection-unsupported-scheme = 仅支持 http 和 https 地址
rejection-credentials = 地址中含有用户名或密码
rejection-duplicate = 重复
rejection-too-many = 一次地址过多
rejection-too-long = 该行过长

proposals-none-waiting = 没有等待决定的建议。
proposals-none-decided = 您尚未对任何建议作出决定。
proposal-shared = 来自其他安装
proposal-confirm = 确认
proposal-reject = 拒绝
proposal-automatic = ATENA 的建议（{ $date }）：{ $licence } · { $terms } · 国家 { $country }
proposal-decided = 决定：{ $licence } · { $terms } · 国家 { $country }
proposal-would-allow = 确认后将允许：
proposal-finding = 核实记录（英文，按原样保存）
proposal-rights-page = 打开许可或条款页面
proposal-confirm-title = 确认“{ $name }”
proposal-reject-title = 拒绝“{ $name }”
proposal-confirmed-crawled = 已确认。该来源将在下次采集时被采集。
proposal-confirmed-not-crawled = 已确认。但合法性原则不允许采集该来源。
proposal-rejected = 已拒绝。从该来源采集的所有内容均已删除。

decision-licence = 许可
decision-terms = 使用条款
decision-country = 适用其法律的国家
decision-reason-confirm = 理由——您阅读了什么以及何时阅读（必填）
decision-reason-reject = 拒绝的理由（必填）
decision-by = 决定人：{ $name }（姓名仅保存在这台电脑上）

share-explain = 列表只包含地址和国家代码——不含任何关于您的信息。ATENA 不会把它发送到任何地方：请复制后按您自己的方式（例如电子邮件）交给 ATENA 编辑，然后将其标记为已交付。
share-nothing = 没有待交付的内容。
share-list = 含 { $count } 个地址的列表
share-copy = 复制
share-copied = 已复制
share-mark-handed-over = 标记为已交付
share-handed-over = 列表已标记为已交付。

receive-explain = 供编辑使用：粘贴其他安装的用户发给您的列表。ATENA 会自行评估每个地址——收到的评估无从证明——并将其保存为建议。
receive-list = 列表（以 ATENA-NAVRHY 1 这一行开头）
receive-action = 读取列表
receive-not-a-list = 这不是 ATENA 列表：第一行必须是 ATENA-NAVRHY 1。
receive-read-before = 此列表之前已读取过（{ $list }）。

## 采集

crawl-title = 采集
crawl-intro = ATENA 访问登记表中的来源和已确认的建议，保存新的和变更的文档，并使其可被搜索。只有在您启动时才会进行采集。
crawl-state-running = 正在采集
crawl-state-stopping = 正在停止——正在完成进行中的检查
crawl-state-stopped = 采集已停止
crawl-running = 正在采集
crawl-start = 开始采集
crawl-stop = 停止
crawl-started = 开始于 { $time }
crawl-checks = 次检查
crawl-hosts = 个已访问的服务器
crawl-due = 个地址等待检查
crawl-store-errors = 个数据库错误
crawl-outcomes = 检查结果
crawl-rules-title = ATENA 如何采集
crawl-rule-robots = 遵守 robots.txt 和发布者的规则（TDMRep、noindex、nosnippet）。
crawl-rule-pace = 对每个服务器都缓慢访问，一个请求接一个请求；服务器要求暂停时就等待。
crawl-rule-open = 只从开放来源获取，只保存法律和许可允许的内容。
crawl-rule-privacy = 以匿名访客身份访问：不使用 cookie、不运行脚本、不加载图片。不发送任何关于您的信息。
crawl-rule-safety = 拒绝程序和含有活动内容的文档，在受限进程中处理文档，不保存原始文件。
crawl-concurrency = 同时处理 { $count } 个服务器（可在设置中更改）。

check-new = 新文档
check-changed = 文档已变更
check-unchanged = 无变化
check-not-modified = 无变化（服务器告知）
check-feed = 已读取 RSS/Atom 订阅
check-sitemap = 已读取网站地图
check-robots = robots.txt 不允许
check-robots-unreachable = 无法获取 robots.txt
check-blocked = 地址不被允许
check-http-error = 服务器错误
check-gone = 文档已不存在
check-network = 网络不可用
check-timeout = 超时
check-redirects = 重定向过多
check-retry-later = 服务器要求等待
check-too-large = 过大
check-unsupported = 不支持的内容类型
check-dangerous = 已拒绝危险内容
check-too-demanding = 已拒绝过于耗费资源的内容
check-extract-error = 无法处理内容
check-feed-error = RSS/Atom 订阅有误
check-sitemap-error = 网站地图有误
check-moved-elsewhere = 已移至其他网站
check-erased = 应请求删除
check-not-permitted = 合法性原则不允许
check-invalid-url = 无效地址
check-store-error = 数据库错误

## 设置

settings-title = 设置
settings-saved = 已保存。
settings-appearance = 语言与外观
settings-language = 界面语言
settings-theme = 外观
settings-theme-system = 跟随系统
settings-theme-light = 浅色
settings-theme-dark = 深色
settings-decisions = 对建议的决定
settings-editor = 您的姓名或昵称
settings-editor-privacy = 它会记录在您的决定中，且只保存在这台电脑上。共享登记表不列出任何个人。
settings-share = 为编辑保留建议的地址
settings-share-explain = ATENA 不会自行发送任何内容；保留的地址在“建议 → 交给编辑”中。
settings-crawler = 采集
settings-concurrency = 同时访问的服务器数（1–16）
settings-concurrency-explain = ATENA 对每个服务器始终一次只发送一个请求；此数字表示同时访问多少个不同的服务器。

erasure-title = 删除请求
erasure-explain = 如果有人要求删除（GDPR 第 17 条），ATENA 会删除有关该地址的一切，并且不再访问它。
erasure-address = 地址
erasure-prefix = 也包括以此开头的所有地址
erasure-reason = 原因（例如：某日的请求）
erasure-action = 删除并屏蔽
erasure-done = 已删除：{ $documents } 份文档和 { $versions } 个版本。该地址已被屏蔽。

about-title = 关于 ATENA
about-version = ATENA { $version }——Apache 2.0 许可下的开源软件，可免费用于个人和商业用途。
about-connections = ATENA 不向任何人发送其用户的任何信息。它不会自行连接任何地方；只有在您启动采集、地址评估或登记表同步时才会联网，而且是以匿名访客的身份。不使用任何第三方服务。
about-pillars = ATENA 的四大支柱
pillar-1 = 严谨、开放，致力于推动每个人的知识进步。
pillar-2 = 匿名与尊重隐私——我们的和他人的。
pillar-3 = 安全，防范危险的人、机器人、代码和流程。
pillar-4 = 遵守法律：在世界任何地方都不得违反任何法律。
about-licence = 所有组成部分的许可证位于应用文件夹“notices”中。
about-repository = GitHub 上的源代码

## 错误

error-detail = 详细信息
error-already-running = ATENA 已在运行。请在已打开的窗口中操作。
error-crawl = 无法准备采集。
error-database = 数据库错误。
error-database-start = 无法启动数据库。
error-database-version = 数据库由与此 ATENA 所附带版本不同的 PostgreSQL 创建。
error-decision-country = ATENA 没有该国家的法律档案。
error-decision-incomplete = 决定需要姓名和理由。
error-decision-licence = 登记表不认识此许可。
error-decision-official = 该国家的官方文本不是自由的；请选择其开放许可。
error-decision-terms = 登记表不认识这些条款。
error-editor-missing = 决定需要您的姓名——请在设置中填写。
error-index = 搜索索引错误。
error-index-foreign = 索引文件夹中的内容不是 ATENA 索引。
error-input-too-long = 文本过长。
error-internal = 发生了意外错误。
error-io = 无法读取或写入文件或文件夹。
error-not-a-list = 这不是 ATENA 列表。
error-not-found = 未找到。
error-not-ready = ATENA 仍在启动。
error-reason-missing = 请填写理由。
error-search = 搜索失败。
error-search-empty = 请输入要查找的内容或选择筛选条件。
error-search-filter = ATENA 不认识此筛选条件。
error-settings = 无法保存设置。
error-shared-register-invalid = 共享登记表无效；未作任何更改。
error-shared-register-unreachable = 无法下载共享登记表。请检查网络连接。
error-url = 这不是有效的网址。
error-private-damaged = 私人账户文件已损坏。
error-private-exists = 这台电脑上已有私人账户。
error-private-locked = 私人部分已锁定——请先登录。
error-private-login = 名称或密码错误。
error-private-name = 名称需为 1 到 40 个字符。
error-private-none = 尚未创建私人账户。
error-private-recovery = 这不是此账户的恢复密钥。
error-private-weak = 密码至少 10 个字符，且不能与名称相同。

## Súkromné / Private

private-group = 私人
private-overview-title = 我的 ATENA
private-overview-of = { $name } 的私人部分——仅保存在这台电脑上。
private-note-title = 私人——仅供您使用
private-note-text = 此部分用您的密码加密，只保存在这台电脑上。您可以在此保存文档，作为供自己使用的副本（依著作权法自然人的私人使用）。请勿分享副本或作其他用途——它们属于作者。
private-all = 全部
private-empty = 此类别中暂无斯洛伐克文档。请开始采集。
private-search-all = 在此类别中搜索
private-create-first = 请先在设置中创建私人账户。
private-lock = 锁定私人部分
private-unlock-title = 登录私人部分
private-unlock = 登录
private-name = 名称
private-password = 密码
private-remember-title = 保持登录？
private-remember-text = 选择“是”，私人部分将保持打开直至您关闭 ATENA；选择“否”，离开后立即锁定。
private-remember-yes = 是，直至关闭 ATENA
private-remember-no = 否，离开后锁定
private-account-title = 私人账户
private-account-intro = 账户只存在于这台电脑上——ATENA 不向任何服务器发送任何内容。密码不会被保存；由它派生的密钥加密您的私人部分。每台电脑一个账户。
private-password-again = 再次输入密码
private-password-rules = 密码至少 10 个字符，且不能与名称相同。创建账户时您会得到恢复密钥——忘记密码时可用它设置新密码。
private-password-mismatch = 两次输入的密码不一致。
private-create = 创建私人账户
private-account-of = 账户 { $name }：
private-state-open = 已打开
private-state-locked = 已锁定
private-change-password = 更改密码
private-forgot = 忘记密码？
private-forgot-text = 输入创建账户时获得的恢复密钥和新密码。您的数据会保留。
private-password-old = 当前密码
private-password-new = 新密码
private-set-password = 设置新密码
private-password-changed = 密码已更改。
private-recovery-title = 恢复密钥
private-recovery-text = 这是您的恢复密钥。请打印或保存在这台电脑以外的安全地方。它只显示这一次——没有它也没有密码，就无法打开私人部分。
private-recovery-copy = 复制
private-recovery-copied = 已复制
private-recovery-kept = 我已保存恢复密钥。
private-recovery-done = 继续
