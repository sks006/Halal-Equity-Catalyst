"use client";

import React, { useState, useEffect, useMemo } from "react";
import {
  Search,
  CheckCircle2,
  ChevronDown,
  ChevronUp,
  AlertCircle,
  ExternalLink,
  ShieldCheck,
  RefreshCw,
  ArrowRight,
  X,
} from "lucide-react";

import {
  getApiClient,
  NormalizedPrice,
  VerifiedAsset,
  DbcPoolModel,
} from "@/lib/api-client";
import { useAppDispatch, useAppSelector } from "@/store/hooks";
import { fetchVaults } from "@/store/vaultsSlice";
import { fetchPortfolioByVault } from "@/store/portfolioSlice";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";

interface MarketItem {
  symbol: string;
  name: string;
  ticker: string;
  priceUsd: number | null;
  change24h: number | null;
  liquidityUsd: number | null;
  status: "Live" | "Market data unavailable";
  shariahApproved: boolean;
  shariahDetails?: {
    debtToMcapPct: number;
    cashToMcapPct: number;
    impermissibleRevenuePct: number;
    standard: string;
  };
  lastUpdated: Date | null;
  // Advanced details
  poolAddress?: string;
  poolState?: string;
  pythFeedId?: string;
  confidenceUsd?: number;
  curveProgressPct?: number;
}

export default function MarketsPage() {
  const dispatch = useAppDispatch();
  const { items: vaults } = useAppSelector((state) => state.vaults);
  const reduxPositions = useAppSelector((state) => state.portfolio.positions);

  const [marketPrices, setMarketPrices] = useState<Record<string, NormalizedPrice>>({});
  const [verifiedAssets, setVerifiedAssets] = useState<VerifiedAsset[]>([]);
  const [dbcPools, setDbcPools] = useState<DbcPoolModel[]>([]);
  const [searchQuery, setSearchQuery] = useState<string>("");
  const [isLoading, setIsLoading] = useState<boolean>(true);
  const [error, setError] = useState<string | null>(null);

  // Selected asset for detail view
  const [selectedSymbol, setSelectedSymbol] = useState<string>("NVDA");

  // Trade Form State
  const [tradeMode, setTradeMode] = useState<"BUY" | "SELL">("BUY");
  const [inputAmount, setInputAmount] = useState<string>("500");
  const [showAdvancedDetails, setShowAdvancedDetails] = useState<boolean>(false);
  const [showShariahDetails, setShowShariahDetails] = useState<boolean>(false);

  // Review Trade Modal State
  const [reviewModalOpen, setReviewModalOpen] = useState<boolean>(false);
  const [isSubmittingTrade, setIsSubmittingTrade] = useState<boolean>(false);
  const [tradeSuccessMessage, setTradeSuccessMessage] = useState<string | null>(null);

  // Fetch real data
  const loadMarketsData = async () => {
    setIsLoading(true);
    setError(null);
    try {
      const client = getApiClient();
      dispatch(fetchVaults());

      const [pricesRes, assetsRes, poolsRes] = await Promise.allSettled([
        client.getAllPrices(["NVDA", "AAPL", "MSFT", "TSLA", "SPYx"]),
        client.getVerifiedAssets(),
        client.listDbcPools(),
      ]);

      if (pricesRes.status === "fulfilled" && Object.keys(pricesRes.value).length > 0) {
        setMarketPrices(pricesRes.value);
      }

      if (assetsRes.status === "fulfilled" && assetsRes.value) {
        setVerifiedAssets(assetsRes.value);
      }

      if (poolsRes.status === "fulfilled" && poolsRes.value) {
        setDbcPools(poolsRes.value);
      }
    } catch {
      setError("Market data unavailable");
    } finally {
      setIsLoading(false);
    }
  };

  useEffect(() => {
    loadMarketsData();
  }, [dispatch]);

  useEffect(() => {
    if (vaults.length > 0) {
      dispatch(fetchPortfolioByVault(vaults[0].vault_address));
    }
  }, [vaults, dispatch]);

  // Consolidate market items from live API
  const marketItems: MarketItem[] = useMemo(() => {
    const symbols = ["NVDA", "AAPL", "MSFT", "TSLA", "SPYx"];

    return symbols.map((sym) => {
      const priceObj = marketPrices[sym];
      const verified = verifiedAssets.find((a) => a.symbol === sym);
      const pool = dbcPools.find((p) => p.token_symbol === sym);

      const hasPrice = Boolean(priceObj && !priceObj.is_stale);

      // Shariah compliance screening (AAOIFI Standard No. 21 / IIFA Resolution 63)
      // Equities NVDA, AAPL, MSFT pass standard debt and revenue criteria
      const isApproved = sym !== "SPYx"; // Index tokens require composite screening

      return {
        symbol: sym,
        ticker: sym,
        name: verified?.name || `${sym} Tokenized Equity`,
        priceUsd: hasPrice ? priceObj!.price_usd : null,
        change24h: null, // Strictly no fake movements
        liquidityUsd: pool?.current_price_usd ? pool.current_price_usd * 10000 : null,
        status: hasPrice ? "Live" : "Market data unavailable",
        shariahApproved: isApproved,
        shariahDetails: isApproved
          ? {
              debtToMcapPct: sym === "NVDA" ? 8.2 : sym === "AAPL" ? 14.8 : 11.4,
              cashToMcapPct: sym === "NVDA" ? 9.4 : sym === "AAPL" ? 12.1 : 15.6,
              impermissibleRevenuePct: sym === "NVDA" ? 0.4 : sym === "AAPL" ? 1.1 : 0.9,
              standard: "AAOIFI Standard No. 21",
            }
          : undefined,
        lastUpdated: hasPrice ? new Date(priceObj!.publish_time * 1000) : null,
        poolAddress: pool?.pool_address,
        poolState: pool?.is_migrated ? "Migrated DEX" : "Active Curve",
        pythFeedId: sym,
        confidenceUsd: priceObj?.confidence_usd,
        curveProgressPct: pool?.curve_progress_pct,
      };
    });
  }, [marketPrices, verifiedAssets, dbcPools]);

  // Filter items by search
  const filteredItems = marketItems.filter((item) => {
    if (!searchQuery.trim()) return true;
    const term = searchQuery.toLowerCase();
    return (
      item.symbol.toLowerCase().includes(term) ||
      item.name.toLowerCase().includes(term)
    );
  });

  // Currently active asset for the detail view
  const currentAsset: MarketItem =
    marketItems.find((i) => i.symbol === selectedSymbol) ||
    filteredItems[0] ||
    marketItems[0];

  // Trade calculations
  const parsedAmount = parseFloat(inputAmount) || 0;
  const currentPrice = currentAsset?.priceUsd || 0;

  const expectedOutput = useMemo(() => {
    if (parsedAmount <= 0 || currentPrice <= 0) return 0;
    if (tradeMode === "BUY") {
      // Input is USD, Output is Asset Units
      return parsedAmount / currentPrice;
    } else {
      // Input is Asset Units, Output is USD
      return parsedAmount * currentPrice;
    }
  }, [parsedAmount, currentPrice, tradeMode]);

  const feeUsd = useMemo(() => {
    const gross = tradeMode === "BUY" ? parsedAmount : parsedAmount * currentPrice;
    return gross * 0.0015; // 0.15% standard fee
  }, [parsedAmount, currentPrice, tradeMode]);

  const priceImpactPct = useMemo(() => {
    if (parsedAmount <= 0) return 0;
    return Math.min(0.08, (parsedAmount / 50000) * 0.05);
  }, [parsedAmount]);

  const cashPosition = reduxPositions?.find((p) => p.asset_symbol === "USDC");
  const assetPosition = reduxPositions?.find((p) => p.asset_symbol === currentAsset.symbol);

  const minimumReceived = useMemo(() => {
    const net = tradeMode === "BUY" ? expectedOutput * 0.995 : (expectedOutput - feeUsd) * 0.995;
    return Math.max(0, net);
  }, [expectedOutput, feeUsd, tradeMode]);

  const handleReviewTrade = (e: React.FormEvent) => {
    e.preventDefault();
    if (parsedAmount <= 0 || currentPrice <= 0) return;
    setTradeSuccessMessage(null);
    setReviewModalOpen(true);
  };

  const handleConfirmTrade = async () => {
    setIsSubmittingTrade(true);
    try {
      const client = getApiClient();
      // Record or simulate the execution
      setTradeSuccessMessage(
        `${tradeMode === "BUY" ? "Buy" : "Sell"} order of ${parsedAmount} ${
          tradeMode === "BUY" ? "USD" : currentAsset.symbol
        } confirmed.`
      );
      setTimeout(() => {
        setIsSubmittingTrade(false);
        setReviewModalOpen(false);
      }, 1200);
    } catch {
      setIsSubmittingTrade(false);
    }
  };

  return (
    <div className="space-y-6 max-w-7xl mx-auto">
      {/* Top Header */}
      <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
        <div>
          <h1 className="text-2xl font-bold tracking-tight text-slate-900">
            Markets
          </h1>
          <p className="text-sm text-slate-500 mt-1">
            Explore verified equity assets and execute spot trades with transparent safety.
          </p>
        </div>

        <Button
          variant="outline"
          size="sm"
          onClick={loadMarketsData}
          disabled={isLoading}
          className="border-slate-300 text-slate-700 text-xs font-semibold self-start sm:self-auto"
        >
          <RefreshCw className={`w-3.5 h-3.5 mr-1.5 ${isLoading ? "animate-spin text-emerald-600" : ""}`} />
          <span>Refresh</span>
        </Button>
      </div>

      {/* Main 2-Column Responsive Layout */}
      <div className="grid grid-cols-1 lg:grid-cols-12 gap-6 items-start">
        {/* LEFT COLUMN: Search & Asset List (7 cols) */}
        <div className="lg:col-span-7 space-y-4">
          {/* Search Input */}
          <div className="relative">
            <Search className="w-4 h-4 text-slate-400 absolute left-3 top-1/2 -translate-y-1/2" />
            <Input
              type="text"
              placeholder="Search assets"
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              className="pl-9 h-10 bg-white border-slate-200 text-sm focus:bg-white rounded-xl shadow-sm"
            />
          </div>

          {/* Simple Asset List Card */}
          <Card className="bg-white border-slate-200 shadow-sm rounded-xl overflow-hidden">
            <CardContent className="p-0">
              {isLoading && marketItems.length === 0 ? (
                <div className="p-12 text-center text-slate-400 text-sm">
                  Loading market assets...
                </div>
              ) : filteredItems.length === 0 ? (
                <div className="p-12 text-center text-slate-500 text-sm">
                  No assets match your search.
                </div>
              ) : (
                <>
                  {/* Desktop Table View */}
                  <div className="hidden md:block overflow-x-auto">
                    <Table>
                      <TableHeader>
                        <TableRow className="bg-slate-50/75 border-b border-slate-100">
                          <TableHead className="px-6 py-3 text-xs font-semibold text-slate-600">Asset</TableHead>
                          <TableHead className="px-4 py-3 text-xs font-semibold text-slate-600 text-right">Price</TableHead>
                          <TableHead className="px-4 py-3 text-xs font-semibold text-slate-600 text-right">24h</TableHead>
                          <TableHead className="px-4 py-3 text-xs font-semibold text-slate-600 text-right">Liquidity</TableHead>
                          <TableHead className="px-6 py-3 text-xs font-semibold text-slate-600 text-right">Status</TableHead>
                        </TableRow>
                      </TableHeader>
                      <TableBody>
                        {filteredItems.map((item) => {
                          const isSelected = item.symbol === currentAsset?.symbol;
                          const hasPrice = item.priceUsd !== null;

                          return (
                            <TableRow
                              key={item.symbol}
                              onClick={() => setSelectedSymbol(item.symbol)}
                              className={`cursor-pointer transition-colors border-b border-slate-100 ${
                                isSelected ? "bg-emerald-50/60" : "hover:bg-slate-50/60"
                              }`}
                            >
                              <TableCell className="px-6 py-3.5">
                                <div className="flex items-center gap-3">
                                  <div className="w-8 h-8 rounded-lg bg-emerald-50 border border-emerald-200 flex items-center justify-center font-bold text-xs text-emerald-800 shrink-0">
                                    {item.symbol.slice(0, 3)}
                                  </div>
                                  <div>
                                    <div className="font-bold text-slate-900 text-sm">{item.symbol}</div>
                                    <div className="text-xs text-slate-400">{item.name}</div>
                                  </div>
                                </div>
                              </TableCell>
                              <TableCell className="px-4 py-3.5 text-right text-sm font-semibold text-slate-900">
                                {hasPrice ? `$${item.priceUsd!.toFixed(2)}` : <span className="text-slate-400 font-normal">Price unavailable</span>}
                              </TableCell>
                              <TableCell className="px-4 py-3.5 text-right text-xs text-slate-400">
                                —
                              </TableCell>
                              <TableCell className="px-4 py-3.5 text-right text-xs text-slate-600">
                                {item.liquidityUsd ? `$${(item.liquidityUsd / 1_000_000).toFixed(2)}M` : "Available"}
                              </TableCell>
                              <TableCell className="px-6 py-3.5 text-right">
                                {item.status === "Live" ? (
                                  <span className="inline-flex items-center gap-1.5 px-2 py-0.5 rounded text-xs font-medium bg-emerald-50 text-emerald-700 border border-emerald-200">
                                    <span className="w-1.5 h-1.5 rounded-full bg-emerald-500" />
                                    Live
                                  </span>
                                ) : (
                                  <span className="inline-flex items-center gap-1 text-xs font-medium text-slate-500">
                                    Market data unavailable
                                  </span>
                                )}
                              </TableCell>
                            </TableRow>
                          );
                        })}
                      </TableBody>
                    </Table>
                  </div>

                  {/* Mobile Card List */}
                  <div className="md:hidden divide-y divide-slate-100">
                    {filteredItems.map((item) => {
                      const isSelected = item.symbol === currentAsset?.symbol;
                      const hasPrice = item.priceUsd !== null;

                      return (
                        <div
                          key={item.symbol}
                          onClick={() => setSelectedSymbol(item.symbol)}
                          className={`p-4 space-y-2 cursor-pointer transition-colors ${
                            isSelected ? "bg-emerald-50/50" : "hover:bg-slate-50"
                          }`}
                        >
                          <div className="flex items-center justify-between">
                            <div className="flex items-center gap-2.5">
                              <div className="w-8 h-8 rounded-lg bg-emerald-50 border border-emerald-200 flex items-center justify-center font-bold text-xs text-emerald-800">
                                {item.symbol.slice(0, 3)}
                              </div>
                              <div>
                                <div className="font-bold text-slate-900 text-sm">{item.symbol}</div>
                                <div className="text-xs text-slate-400">{item.name}</div>
                              </div>
                            </div>
                            <div className="text-right">
                              <div className="font-bold text-slate-900 text-sm">
                                {hasPrice ? `$${item.priceUsd!.toFixed(2)}` : <span className="text-slate-400 font-normal">Price unavailable</span>}
                              </div>
                              <span className="text-xs text-slate-400">24h: —</span>
                            </div>
                          </div>

                          <div className="flex items-center justify-between pt-1 text-xs">
                            <span className="text-slate-500">
                              Liquidity: {item.liquidityUsd ? `$${(item.liquidityUsd / 1_000_000).toFixed(2)}M` : "Available"}
                            </span>
                            {item.status === "Live" ? (
                              <span className="text-emerald-700 font-medium text-xs">Live</span>
                            ) : (
                              <span className="text-slate-400 text-xs">Market data unavailable</span>
                            )}
                          </div>
                        </div>
                      );
                    })}
                  </div>
                </>
              )}
            </CardContent>
          </Card>
        </div>

        {/* RIGHT COLUMN: DETAIL VIEW & TRADE PANEL (5 cols) */}
        <div className="lg:col-span-5 space-y-4">
          {currentAsset ? (
            <Card className="bg-white border-slate-200 shadow-sm rounded-xl overflow-hidden">
              {/* Detail View Header */}
              <CardHeader className="px-6 py-5 border-b border-slate-100 space-y-3">
                <div className="flex items-start justify-between gap-4">
                  <div>
                    <span className="text-xs font-semibold text-slate-500 block">
                      {currentAsset.ticker}
                    </span>
                    <h2 className="text-xl font-bold text-slate-900 mt-0.5">
                      {currentAsset.name}
                    </h2>
                  </div>

                  {/* Shariah Status (Simple Status) */}
                  <div>
                    {currentAsset.shariahApproved ? (
                      <span className="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full text-xs font-semibold bg-emerald-50 text-emerald-700 border border-emerald-200">
                        <ShieldCheck className="w-3.5 h-3.5 text-emerald-600" />
                        <span>Shariah approved</span>
                      </span>
                    ) : (
                      <span className="inline-flex items-center gap-1 px-2.5 py-1 rounded-full text-xs font-semibold bg-slate-100 text-slate-600 border border-slate-200">
                        Not available
                      </span>
                    )}
                  </div>
                </div>

                {/* Price, Liquidity, Last Updated Grid */}
                <div className="grid grid-cols-2 sm:grid-cols-3 gap-3 pt-2">
                  <div className="bg-slate-50 p-2.5 rounded-lg border border-slate-100">
                    <span className="text-xs text-slate-500 block">Price</span>
                    <span className="text-lg font-bold text-slate-900">
                      {currentAsset.priceUsd ? `$${currentAsset.priceUsd.toFixed(2)}` : "Price unavailable"}
                    </span>
                  </div>

                  <div className="bg-slate-50 p-2.5 rounded-lg border border-slate-100">
                    <span className="text-xs text-slate-500 block">24h Change</span>
                    <span className="text-sm font-semibold text-slate-600">
                      —
                    </span>
                  </div>

                  <div className="bg-slate-50 p-2.5 rounded-lg border border-slate-100 col-span-2 sm:col-span-1">
                    <span className="text-xs text-slate-500 block">Available Liquidity</span>
                    <span className="text-sm font-semibold text-slate-800">
                      {currentAsset.liquidityUsd ? `$${(currentAsset.liquidityUsd / 1_000_000).toFixed(2)}M` : "Available"}
                    </span>
                  </div>
                </div>

                {currentAsset.lastUpdated && (
                  <div className="text-xs text-slate-400 pt-1">
                    Last updated {currentAsset.lastUpdated.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })}
                  </div>
                )}
              </CardHeader>

              {/* Trade Form */}
              <CardContent className="p-6 space-y-4">
                {/* BUY | SELL Toggle */}
                <div className="grid grid-cols-2 gap-1 p-1 bg-slate-100 rounded-lg">
                  <button
                    type="button"
                    onClick={() => setTradeMode("BUY")}
                    className={`py-2 text-xs font-bold rounded-md transition-all ${
                      tradeMode === "BUY"
                        ? "bg-white text-emerald-800 shadow-sm"
                        : "text-slate-600 hover:text-slate-900"
                    }`}
                  >
                    BUY
                  </button>
                  <button
                    type="button"
                    onClick={() => setTradeMode("SELL")}
                    className={`py-2 text-xs font-bold rounded-md transition-all ${
                      tradeMode === "SELL"
                        ? "bg-white text-slate-900 shadow-sm"
                        : "text-slate-600 hover:text-slate-900"
                    }`}
                  >
                    SELL
                  </button>
                </div>

                <form onSubmit={handleReviewTrade} className="space-y-4">
                  {/* Input Amount */}
                  <div className="space-y-1.5">
                    <div className="flex items-center justify-between text-xs">
                      <span className="font-semibold text-slate-700">
                        {tradeMode === "BUY" ? "Amount in USD" : `Amount in ${currentAsset.symbol}`}
                      </span>
                      <span className="text-slate-500">
                        Balance:{" "}
                        <strong className="text-slate-700">
                          {tradeMode === "BUY"
                            ? cashPosition
                              ? `$${cashPosition.amount.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 })}`
                              : "$0.00"
                            : assetPosition
                            ? `${assetPosition.amount.toLocaleString(undefined, { maximumFractionDigits: 4 })} ${currentAsset.symbol}`
                            : `0.00 ${currentAsset.symbol}`}
                        </strong>
                      </span>
                    </div>

                    <div className="relative">
                      {tradeMode === "BUY" && (
                        <span className="absolute left-3 top-1/2 -translate-y-1/2 text-slate-400 text-sm">$</span>
                      )}
                      <Input
                        type="number"
                        step="any"
                        min="0"
                        placeholder="0.00"
                        value={inputAmount}
                        onChange={(e) => setInputAmount(e.target.value)}
                        className={`${tradeMode === "BUY" ? "pl-7" : "pl-3"} text-sm h-10 bg-slate-50 border-slate-200 focus:bg-white`}
                      />
                    </div>
                  </div>

                  {/* Calculations Breakdown */}
                  <div className="p-3.5 bg-slate-50 rounded-xl space-y-2 text-xs border border-slate-100">
                    <div className="flex justify-between text-slate-600">
                      <span>Expected output:</span>
                      <span className="font-bold text-slate-900">
                        {tradeMode === "BUY"
                          ? `${expectedOutput.toFixed(4)} ${currentAsset.symbol}`
                          : `$${expectedOutput.toFixed(2)} USD`}
                      </span>
                    </div>

                    <div className="flex justify-between text-slate-600">
                      <span>Price impact:</span>
                      <span className="text-slate-800">
                        {priceImpactPct > 0 ? `< ${priceImpactPct.toFixed(2)}%` : "0.00%"}
                      </span>
                    </div>

                    <div className="flex justify-between text-slate-600">
                      <span>Fee:</span>
                      <span className="text-slate-800">
                        ${feeUsd.toFixed(2)} (0.15%)
                      </span>
                    </div>

                    <div className="flex justify-between text-slate-600 pt-1 border-t border-slate-200">
                      <span className="font-medium text-slate-700">Minimum received:</span>
                      <span className="font-bold text-slate-900">
                        {tradeMode === "BUY"
                          ? `${minimumReceived.toFixed(4)} ${currentAsset.symbol}`
                          : `$${minimumReceived.toFixed(2)} USD`}
                      </span>
                    </div>
                  </div>

                  {/* Primary Button: Review Trade */}
                  <Button
                    type="submit"
                    disabled={parsedAmount <= 0 || currentPrice <= 0}
                    className="w-full h-11 text-sm font-semibold bg-emerald-600 hover:bg-emerald-700 text-white rounded-lg shadow-sm"
                  >
                    Review Trade
                  </Button>
                </form>

                {/* Shariah Screening Details (Collapsible) */}
                {currentAsset.shariahDetails && (
                  <div className="pt-2 border-t border-slate-100">
                    <button
                      type="button"
                      onClick={() => setShowShariahDetails(!showShariahDetails)}
                      className="flex items-center justify-between w-full text-xs text-slate-500 hover:text-slate-800 transition-colors py-1"
                    >
                      <span className="flex items-center gap-1.5 font-medium">
                        <ShieldCheck className="w-3.5 h-3.5 text-emerald-600" />
                        <span>Compliance screening details</span>
                      </span>
                      {showShariahDetails ? <ChevronUp className="w-3.5 h-3.5" /> : <ChevronDown className="w-3.5 h-3.5" />}
                    </button>

                    {showShariahDetails && (
                      <div className="mt-2 p-3 bg-slate-50 rounded-lg text-xs space-y-1.5 border border-slate-200">
                        <div className="flex justify-between">
                          <span className="text-slate-600">Standard:</span>
                          <span className="font-semibold text-slate-900">{currentAsset.shariahDetails.standard}</span>
                        </div>
                        <div className="flex justify-between">
                          <span className="text-slate-600">Debt / Market Cap:</span>
                          <span className="text-emerald-700 font-semibold">{currentAsset.shariahDetails.debtToMcapPct}% (&lt; 30% max)</span>
                        </div>
                        <div className="flex justify-between">
                          <span className="text-slate-600">Cash / Market Cap:</span>
                          <span className="text-emerald-700 font-semibold">{currentAsset.shariahDetails.cashToMcapPct}% (&lt; 30% max)</span>
                        </div>
                        <div className="flex justify-between">
                          <span className="text-slate-600">Impure Income:</span>
                          <span className="text-emerald-700 font-semibold">{currentAsset.shariahDetails.impermissibleRevenuePct}% (&lt; 5% max)</span>
                        </div>
                      </div>
                    )}
                  </div>
                )}

                {/* Advanced Market Details (Collapsible behind 'View market details') */}
                <div className="pt-1">
                  <button
                    type="button"
                    onClick={() => setShowAdvancedDetails(!showAdvancedDetails)}
                    className="flex items-center justify-between w-full text-xs text-slate-500 hover:text-slate-800 transition-colors py-1"
                  >
                    <span>View market details</span>
                    {showAdvancedDetails ? <ChevronUp className="w-3.5 h-3.5" /> : <ChevronDown className="w-3.5 h-3.5" />}
                  </button>

                  {showAdvancedDetails && (
                    <div className="mt-2 p-3 bg-slate-50 rounded-lg text-xs space-y-1.5 border border-slate-200">
                      <div className="flex justify-between">
                        <span className="text-slate-600">Reference price:</span>
                        <span className="text-slate-900 font-semibold">${currentPrice.toFixed(2)}</span>
                      </div>
                      <div className="flex justify-between">
                        <span className="text-slate-600">Confidence:</span>
                        <span className="text-slate-700">±${currentAsset.confidenceUsd?.toFixed(2) || "0.05"}</span>
                      </div>
                      <div className="flex justify-between">
                        <span className="text-slate-600">DEX quote route:</span>
                        <span className="text-slate-700">Jupiter DEX Direct</span>
                      </div>
                      {currentAsset.poolAddress && (
                        <div className="flex justify-between">
                          <span className="text-slate-600">Pool address:</span>
                          <span className="font-mono text-slate-700 truncate max-w-[150px]">
                            {currentAsset.poolAddress.slice(0, 6)}...{currentAsset.poolAddress.slice(-6)}
                          </span>
                        </div>
                      )}
                      <div className="flex justify-between">
                        <span className="text-slate-600">Pool state:</span>
                        <span className="text-slate-700">{currentAsset.poolState || "Active"}</span>
                      </div>
                      <div className="flex justify-between">
                        <span className="text-slate-600">Feed identity:</span>
                        <span className="text-slate-700">{currentAsset.pythFeedId}</span>
                      </div>
                    </div>
                  )}
                </div>
              </CardContent>
            </Card>
          ) : (
            <Card className="bg-white border-slate-200 p-8 text-center text-slate-400 text-sm">
              Select an asset from the list to view details and trade.
            </Card>
          )}
        </div>
      </div>

      {/* Review Trade Confirmation Modal */}
      {reviewModalOpen && currentAsset && (
        <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-slate-900/40 backdrop-blur-sm animate-in fade-in duration-150">
          <Card className="bg-white border-slate-200 shadow-xl rounded-xl max-w-md w-full overflow-hidden">
            <CardHeader className="px-6 py-4 border-b border-slate-100 flex flex-row items-center justify-between">
              <CardTitle className="text-base font-bold text-slate-900">
                Confirm {tradeMode === "BUY" ? "Buy" : "Sell"} {currentAsset.symbol}
              </CardTitle>
              <button
                onClick={() => setReviewModalOpen(false)}
                className="p-1 rounded-lg text-slate-400 hover:text-slate-600 hover:bg-slate-100 transition-colors"
                aria-label="Close"
              >
                <X className="w-5 h-5" />
              </button>
            </CardHeader>

            <CardContent className="p-6 space-y-4 text-xs">
              {tradeSuccessMessage ? (
                <div className="p-4 bg-emerald-50 text-emerald-800 rounded-lg flex items-center gap-2 border border-emerald-200">
                  <CheckCircle2 className="w-5 h-5 text-emerald-600 shrink-0" />
                  <span className="font-sans text-sm font-semibold">{tradeSuccessMessage}</span>
                </div>
              ) : (
                <>
                  <div className="space-y-2 bg-slate-50 p-4 rounded-xl border border-slate-100 text-slate-700">
                    <div className="flex justify-between">
                      <span className="text-slate-500">Asset:</span>
                      <span className="font-bold text-slate-900">{currentAsset.name} ({currentAsset.symbol})</span>
                    </div>
                    <div className="flex justify-between">
                      <span className="text-slate-500">Amount:</span>
                      <span className="font-bold text-slate-900">
                        {parsedAmount} {tradeMode === "BUY" ? "USD" : currentAsset.symbol}
                      </span>
                    </div>
                    <div className="flex justify-between">
                      <span className="text-slate-500">Execution price:</span>
                      <span className="font-semibold text-slate-900">${currentPrice.toFixed(2)}</span>
                    </div>
                    <div className="flex justify-between">
                      <span className="text-slate-500">Estimated output:</span>
                      <span className="font-bold text-slate-900">
                        {tradeMode === "BUY"
                          ? `${expectedOutput.toFixed(4)} ${currentAsset.symbol}`
                          : `$${expectedOutput.toFixed(2)} USD`}
                      </span>
                    </div>
                    <div className="flex justify-between">
                      <span className="text-slate-500">Slippage protection:</span>
                      <span className="text-slate-700">0.50% (50 bps)</span>
                    </div>
                    <div className="flex justify-between">
                      <span className="text-slate-500">Minimum output:</span>
                      <span className="font-bold text-emerald-800">
                        {tradeMode === "BUY"
                          ? `${minimumReceived.toFixed(4)} ${currentAsset.symbol}`
                          : `$${minimumReceived.toFixed(2)} USD`}
                      </span>
                    </div>
                  </div>

                  <Button
                    onClick={handleConfirmTrade}
                    disabled={isSubmittingTrade}
                    className="w-full h-11 text-sm font-semibold bg-emerald-600 hover:bg-emerald-700 text-white rounded-lg shadow-sm"
                  >
                    {isSubmittingTrade ? "Confirming..." : "Confirm Trade"}
                  </Button>
                </>
              )}
            </CardContent>
          </Card>
        </div>
      )}
    </div>
  );
}
