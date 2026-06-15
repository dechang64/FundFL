# ── python/tests/test_risk_analyzer.py ──
"""Tests for FundFL Risk Analyzer."""

import sys, os
sys.path.insert(0, os.path.dirname(os.path.dirname(__file__)))

import pytest
import numpy as np
from analysis.risk_analyzer import RiskAnalyzer


class TestRiskAnalyzer:
    def test_basic_computation(self):
        analyzer = RiskAnalyzer()
        np.random.seed(42)
        returns = np.random.normal(0.0005, 0.015, 252)  # ~12% annual, ~24% vol
        profile = analyzer.compute(returns, fund_code="000001", fund_name="Test Fund")

        assert profile.fund_code == "000001"
        assert profile.annual_return > 0
        assert profile.annual_volatility > 0
        assert profile.sharpe_ratio != 0
        assert profile.max_drawdown < 0
        assert 0 <= profile.win_rate <= 1

    def test_feature_vector(self):
        analyzer = RiskAnalyzer()
        returns = np.random.normal(0.001, 0.02, 252)
        profile = analyzer.compute(returns)
        vec = profile.to_feature_vector()
        assert len(vec) == 16
        assert vec.dtype == np.float32

    def test_with_benchmark(self):
        analyzer = RiskAnalyzer()
        np.random.seed(42)
        returns = np.random.normal(0.001, 0.02, 252)
        benchmark = np.random.normal(0.0005, 0.015, 252)
        profile = analyzer.compute(returns, benchmark=benchmark)
        assert profile.beta != 0
        assert profile.jensen_alpha != 0
        assert profile.tracking_error > 0

    def test_similarity(self):
        analyzer = RiskAnalyzer()
        np.random.seed(42)
        r1 = np.random.normal(0.001, 0.02, 252)
        r2 = np.random.normal(0.001, 0.02, 252)
        r3 = np.random.normal(-0.001, 0.05, 252)
        p1 = analyzer.compute(r1)
        p2 = analyzer.compute(r2)
        p3 = analyzer.compute(r3)
        sim_12 = analyzer.similarity(p1, p2)
        sim_13 = analyzer.similarity(p1, p3)
        # Similar funds should be more similar than dissimilar ones
        assert sim_12 > sim_13

    def test_to_dict(self):
        analyzer = RiskAnalyzer()
        returns = np.random.normal(0.001, 0.02, 252)
        profile = analyzer.compute(returns, fund_code="000002")
        d = profile.to_dict()
        assert d["fund_code"] == "000002"
        assert "sharpe_ratio" in d

    def test_short_series(self):
        analyzer = RiskAnalyzer()
        returns = np.array([0.01, -0.02, 0.03, -0.01, 0.02])
        profile = analyzer.compute(returns)
        assert profile is not None
        assert not np.isnan(profile.annual_return)


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
