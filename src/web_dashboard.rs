use axum::{
    http::StatusCode,
    response::{Html, IntoResponse},
    routing::get,
    Router,
};

/// 创建 Web Dashboard 路由
pub fn create_dashboard() -> Router {
    Router::new()
        .route("/", get(index))
        .route("/fund/{code}", get(fund_detail))
}

/// 首页：基金列表 + 统计
async fn index() -> Html<String> {
    Html(DASHBOARD_HTML.to_string())
}

/// 基金详情页
async fn fund_detail() -> Html<String> {
    Html(FUND_DETAIL_HTML.to_string())
}

const DASHBOARD_HTML: &str = r##"<!DOCTYPE html>
<html lang="zh-CN">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>FundFL — 私募基金分析平台</title>
    <style>
        * { margin: 0; padding: 0; box-sizing: border-box; }
        body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif; background: #0f172a; color: #e2e8f0; }
        .container { max-width: 1200px; margin: 0 auto; padding: 20px; }
        header { display: flex; justify-content: space-between; align-items: center; padding: 20px 0; border-bottom: 1px solid #1e293b; margin-bottom: 24px; }
        h1 { font-size: 24px; color: #38bdf8; }
        h1 span { color: #94a3b8; font-size: 14px; font-weight: normal; margin-left: 12px; }
        .stats { display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: 16px; margin-bottom: 24px; }
        .stat-card { background: #1e293b; border-radius: 12px; padding: 20px; border: 1px solid #334155; }
        .stat-card .label { font-size: 12px; color: #64748b; text-transform: uppercase; letter-spacing: 1px; }
        .stat-card .value { font-size: 28px; font-weight: 700; color: #38bdf8; margin-top: 4px; }
        .controls { display: flex; gap: 12px; margin-bottom: 20px; flex-wrap: wrap; }
        select, input { background: #1e293b; color: #e2e8f0; border: 1px solid #334155; border-radius: 8px; padding: 8px 12px; font-size: 14px; }
        input { flex: 1; min-width: 200px; }
        button { background: #0ea5e9; color: white; border: none; border-radius: 8px; padding: 8px 20px; cursor: pointer; font-size: 14px; }
        button:hover { background: #0284c7; }
        table { width: 100%; border-collapse: collapse; }
        th { text-align: left; padding: 12px; font-size: 12px; color: #64748b; text-transform: uppercase; letter-spacing: 1px; border-bottom: 1px solid #1e293b; }
        td { padding: 12px; border-bottom: 1px solid #1e293b; }
        tr:hover { background: #1e293b; }
        .badge { display: inline-block; padding: 2px 8px; border-radius: 4px; font-size: 12px; }
        .badge-pos { background: #064e3b; color: #34d399; }
        .badge-neg { background: #7f1d1d; color: #f87171; }
        .badge-cat { background: #1e3a5f; color: #60a5fa; }
        a { color: #38bdf8; text-decoration: none; }
        a:hover { text-decoration: underline; }
        .loading { text-align: center; padding: 40px; color: #64748b; }
        .footer { text-align: center; padding: 40px 0 20px; color: #475569; font-size: 12px; }
    </style>
</head>
<body>
    <div class="container">
        <header>
            <h1>FundFL <span>开源私募基金分析平台 v0.1.0</span></h1>
            <div>
                <a href="https://github.com/dechang64/fundfl" target="_blank">GitHub</a>
            </div>
        </header>

        <div class="stats" id="stats">
            <div class="stat-card"><div class="label">基金总数</div><div class="value" id="total-funds">—</div></div>
            <div class="stat-card"><div class="label">向量索引</div><div class="value" id="total-vectors">—</div></div>
            <div class="stat-card"><div class="label">审计链</div><div class="value" id="audit-chain">—</div></div>
            <div class="stat-card"><div class="label">数据完整性</div><div class="value" id="audit-valid">—</div></div>
        </div>

        <div class="controls">
            <input type="text" id="search-input" placeholder="输入基金代码搜索相似基金（如 PXSGX）">
            <button onclick="searchSimilar()">搜索相似</button>
            <select id="category-filter" onchange="loadFunds()">
                <option value="">全部类别</option>
            </select>
            <select id="sort-by" onchange="loadFunds()">
                <option value="sharpe">Sharpe</option>
                <option value="alpha">Alpha</option>
                <option value="return">年化收益</option>
                <option value="name">名称</option>
            </select>
        </div>

        <table>
            <thead>
                <tr>
                    <th>代码</th>
                    <th>名称</th>
                    <th>类别</th>
                    <th>Sharpe</th>
                    <th>Alpha</th>
                    <th>年化收益</th>
                    <th>最大回撤</th>
                    <th>操作</th>
                </tr>
            </thead>
            <tbody id="fund-table">
                <tr><td colspan="8" class="loading">加载中...</td></tr>
            </tbody>
        </table>

        <div class="footer">
            FundFL — 开源私募基金数据分析与资产定价平台 | Rust + HNSW + gRPC | MIT License
        </div>
    </div>

    <script>
        const API = window.location.origin + '/api/v1';

        async function loadStats() {
            const res = await fetch(API + '/stats');
            const data = await res.json();
            document.getElementById('total-funds').textContent = data.total_funds;
            document.getElementById('total-vectors').textContent = data.total_vectors;
            document.getElementById('audit-chain').textContent = data.audit_chain_length;
            document.getElementById('audit-valid').textContent = data.audit_chain_valid ? '✅' : '❌';

            const sel = document.getElementById('category-filter');
            data.categories.forEach(c => {
                const opt = document.createElement('option');
                opt.value = c.category;
                opt.textContent = c.category_cn + ' (' + c.count + ')';
                sel.appendChild(opt);
            });
        }

        async function loadFunds() {
            const category = document.getElementById('category-filter').value;
            const sortBy = document.getElementById('sort-by').value;
            const url = API + '/funds?sort_by=' + sortBy + '&sort_order=desc&page_size=50' + (category ? '&category=' + category : '');
            const res = await fetch(url);
            const data = await res.json();
            const tbody = document.getElementById('fund-table');

            if (data.funds.length === 0) {
                tbody.innerHTML = '<tr><td colspan="8" class="loading">暂无数据</td></tr>';
                return;
            }

            tbody.innerHTML = data.funds.map(f => {
                const risk = f.risk || {};
                const sharpeClass = (risk.sharpe || 0) > 0 ? 'badge-pos' : 'badge-neg';
                const alphaClass = (risk.alpha || 0) > 0 ? 'badge-pos' : 'badge-neg';
                return '<tr>' +
                    '<td><a href="/fund/' + f.code + '">' + f.code + '</a></td>' +
                    '<td>' + f.name + '</td>' +
                    '<td><span class="badge badge-cat">' + (f.category_cn || f.category) + '</span></td>' +
                    '<td><span class="badge ' + sharpeClass + '">' + (risk.sharpe || '—').toFixed(4) + '</span></td>' +
                    '<td><span class="badge ' + alphaClass + '">' + (risk.alpha || '—').toFixed(4) + '</span></td>' +
                    '<td>' + ((risk.ann_return || 0) * 100).toFixed(2) + '%</td>' +
                    '<td>' + ((risk.max_drawdown || 0) * 100).toFixed(2) + '%</td>' +
                    '<td><button onclick="searchSimilar(\'' + f.code + '\')">相似</button></td>' +
                    '</tr>';
            }).join('');
        }

        async function searchSimilar(code) {
            code = code || document.getElementById('search-input').value.trim().toUpperCase();
            if (!code) return;
            const res = await fetch(API + '/funds/' + code + '/similar');
            if (!res.ok) { alert('基金 ' + code + ' 未找到或无风险数据'); return; }
            const data = await res.json();
            alert('与 ' + code + ' 最相似的基金:\n\n' + data.similar.map((s, i) =>
                (i+1) + '. ' + s.fund_code + ' (' + s.name + ') - 距离: ' + s.distance.toFixed(4) + ', Sharpe: ' + s.sharpe.toFixed(4)
            ).join('\n'));
        }

        loadStats();
        loadFunds();
    </script>
</body>
</html>"##;

const FUND_DETAIL_HTML: &str = r##"<!DOCTYPE html>
<html lang="zh-CN">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>基金详情 — FundFL</title>
    <style>
        * { margin: 0; padding: 0; box-sizing: border-box; }
        body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif; background: #0f172a; color: #e2e8f0; }
        .container { max-width: 900px; margin: 0 auto; padding: 20px; }
        h1 { color: #38bdf8; margin-bottom: 8px; }
        .subtitle { color: #64748b; margin-bottom: 24px; }
        .grid { display: grid; grid-template-columns: 1fr 1fr; gap: 16px; margin-bottom: 24px; }
        .card { background: #1e293b; border-radius: 12px; padding: 20px; border: 1px solid #334155; }
        .card h3 { font-size: 14px; color: #64748b; margin-bottom: 12px; text-transform: uppercase; }
        .metric { display: flex; justify-content: space-between; padding: 8px 0; border-bottom: 1px solid #0f172a; }
        .metric:last-child { border-bottom: none; }
        .metric .label { color: #94a3b8; }
        .metric .value { font-weight: 600; }
        .pos { color: #34d399; }
        .neg { color: #f87171; }
        a { color: #38bdf8; }
        .similar-table { width: 100%; border-collapse: collapse; margin-top: 12px; }
        .similar-table th, .similar-table td { padding: 8px 12px; text-align: left; border-bottom: 1px solid #0f172a; }
        .loading { text-align: center; padding: 40px; color: #64748b; }
    </style>
</head>
<body>
    <div class="container">
        <a href="/">← 返回列表</a>
        <h1 id="fund-name">加载中...</h1>
        <div class="subtitle" id="fund-code"></div>

        <div class="grid">
            <div class="card">
                <h3>基本信息</h3>
                <div id="fund-info"><div class="loading">加载中...</div></div>
            </div>
            <div class="card">
                <h3>风险指标</h3>
                <div id="risk-info"><div class="loading">加载中...</div></div>
            </div>
        </div>

        <div class="card">
            <h3>相似基金（向量检索）</h3>
            <div id="similar-info"><div class="loading">加载中...</div></div>
        </div>
    </div>

    <script>
        const API = window.location.origin + '/api/v1';
        const code = window.location.pathname.split('/').pop();

        async function load() {
            const res = await fetch(API + '/funds/' + code);
            if (!res.ok) { document.getElementById('fund-name').textContent = '基金未找到'; return; }
            const data = await res.json();

            document.getElementById('fund-name').textContent = data.fund.name;
            document.getElementById('fund-code').textContent = data.fund.code + ' · ' + (data.fund.category_cn || data.fund.category);

            document.getElementById('fund-info').innerHTML =
                metric('基金代码', data.fund.code) +
                metric('类别', data.fund.category_cn || data.fund.category) +
                metric('基金经理', data.fund.manager || '—') +
                metric('基金公司', data.fund.company || '—') +
                metric('最新净值', data.fund.nav) +
                metric('累计净值', data.fund.acc_nav) +
                metric('数据月数', data.fund.data_months);

            if (data.risk) {
                const r = data.risk;
                document.getElementById('risk-info').innerHTML =
                    metric('年化收益', (r.ann_return * 100).toFixed(2) + '%', r.ann_return > 0) +
                    metric('年化波动', (r.ann_vol * 100).toFixed(2) + '%') +
                    metric('Sharpe', r.sharpe.toFixed(4), r.sharpe > 0) +
                    metric('Sortino', r.sortino.toFixed(4), r.sortino > 0) +
                    metric('Jensen Alpha', r.alpha.toFixed(4), r.alpha > 0) +
                    metric('Beta', r.beta.toFixed(4)) +
                    metric('Treynor', r.treynor.toFixed(4), r.treynor > 0) +
                    metric('最大回撤', (r.max_drawdown * 100).toFixed(2) + '%', false) +
                    metric('VaR(95%)', (r.var_95 * 100).toFixed(2) + '%', false) +
                    metric('CVaR(95%)', (r.cvar_95 * 100).toFixed(2) + '%', false) +
                    metric('M²', r.m2.toFixed(4), r.m2 > 0) +
                    metric('信息比率', r.info_ratio.toFixed(4), r.info_ratio > 0) +
                    metric('Calmar', r.calmar.toFixed(4), r.calmar > 0) +
                    metric('偏度', r.skewness.toFixed(4)) +
                    metric('峰度', r.kurtosis.toFixed(4)) +
                    metric('胜率', (r.win_rate * 100).toFixed(1) + '%');
            } else {
                document.getElementById('risk-info').innerHTML = '<div class="loading">暂无风险数据</div>';
            }

            // 加载相似基金
            const simRes = await fetch(API + '/funds/' + code + '/similar');
            if (simRes.ok) {
                const simData = await simRes.json();
                if (simData.similar.length > 0) {
                    document.getElementById('similar-info').innerHTML =
                        '<table class="similar-table"><thead><tr><th>排名</th><th>代码</th><th>名称</th><th>距离</th><th>Sharpe</th></tr></thead><tbody>' +
                        simData.similar.map((s, i) =>
                            '<tr><td>' + (i+1) + '</td><td><a href="/fund/' + s.fund_code + '">' + s.fund_code + '</a></td><td>' + s.name + '</td><td>' + s.distance.toFixed(4) + '</td><td>' + s.sharpe.toFixed(4) + '</td></tr>'
                        ).join('') + '</tbody></table>';
                } else {
                    document.getElementById('similar-info').innerHTML = '<div class="loading">暂无相似基金</div>';
                }
            }
        }

        function metric(label, value, positive) {
            const cls = positive === undefined ? '' : (positive ? ' pos' : ' neg');
            return '<div class="metric"><span class="label">' + label + '</span><span class="value' + cls + '">' + value + '</span></div>';
        }

        load();
    </script>
</body>
</html>"##;
