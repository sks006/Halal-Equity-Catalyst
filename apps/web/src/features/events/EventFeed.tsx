"use client";

import React, { useState } from "react";
import {
  Activity,
  ArrowUpRight,
  CheckCircle2,
  Clock,
  Filter,
  Flame,
  Globe,
  Play,
  Radio,
  Search,
  Sparkles,
  TrendingDown,
  TrendingUp,
  Zap,
} from "lucide-react";
import { EventModel } from "@equity-catalyst/sdk";
import { useAppDispatch, useAppSelector } from "../../store/hooks";
import {
  setSelectedEvent,
  startReplay,
  setFilterType,
} from "../../store/eventsSlice";
import { Card, CardContent, CardHeader, CardTitle } from "../../components/ui/card";
import { Badge } from "../../components/ui/badge";
import { Button } from "../../components/ui/button";
import { Input } from "../../components/ui/input";

interface EventFeedProps {
  onSelectEvent?: (eventId: string) => void;
  className?: string;
}

export function EventFeed({ onSelectEvent, className }: EventFeedProps) {
  const dispatch = useAppDispatch();
  const { events, selectedEventId, filterType, isPlaying } = useAppSelector(
    (state) => state.events
  );
  const [searchQuery, setSearchQuery] = useState("");

  const handleSelect = (id: string) => {
    dispatch(setSelectedEvent(id));
    if (onSelectEvent) onSelectEvent(id);
  };

  const handleReplayClick = () => {
    // Select the NVDA earnings event and start playback
    dispatch(setSelectedEvent("a8b1c2d3-e4f5-4678-90ab-cdef12345678"));
    dispatch(startReplay());
  };

  const filteredEvents = events.filter((ev) => {
    if (filterType !== "ALL" && ev.event_type !== filterType) return false;
    if (!searchQuery) return true;

    const q = searchQuery.toLowerCase();
    const typeMatch = ev.event_type.toLowerCase().includes(q);
    const sourceMatch = ev.source.toLowerCase().includes(q);
    const payloadStr = JSON.stringify(ev.payload).toLowerCase();
    return typeMatch || sourceMatch || payloadStr.includes(q);
  });

  const getEventTypeBadge = (type: string) => {
    switch (type) {
      case "EARNINGS_BEAT":
        return (
          <Badge variant="success" className="gap-1 font-mono text-[10px]">
            <Sparkles className="w-3 h-3 text-emerald-600" />
            <span>EARNINGS BEAT</span>
          </Badge>
        );
      case "DRIFT_REBALANCE":
        return (
          <Badge variant="cyan" className="gap-1 font-mono text-[10px]">
            <Activity className="w-3 h-3 text-cyan-600" />
            <span>DRIFT REBALANCE</span>
          </Badge>
        );
      case "DEPOSIT_DETECTED":
        return (
          <Badge variant="secondary" className="gap-1 font-mono text-[10px]">
            <ArrowUpRight className="w-3 h-3 text-slate-700" />
            <span>ON-CHAIN DEPOSIT</span>
          </Badge>
        );
      case "NEWS_VOLATILITY":
        return (
          <Badge variant="warning" className="gap-1 font-mono text-[10px]">
            <Radio className="w-3 h-3 text-amber-600" />
            <span>MARKET NEWS</span>
          </Badge>
        );
      default:
        return (
          <Badge variant="outline" className="font-mono text-[10px]">
            {type}
          </Badge>
        );
    }
  };

  const getSentimentBadge = (score: number | null | undefined) => {
    if (score === null || score === undefined) return null;
    if (score >= 0.5) {
      return (
        <span className="inline-flex items-center gap-1 text-[11px] font-semibold text-emerald-700 bg-emerald-50 px-2 py-0.5 rounded-full border border-emerald-200 font-mono">
          <TrendingUp className="w-3 h-3" />
          +{(score * 100).toFixed(0)} Bullish
        </span>
      );
    }
    if (score <= -0.2) {
      return (
        <span className="inline-flex items-center gap-1 text-[11px] font-semibold text-rose-700 bg-rose-50 px-2 py-0.5 rounded-full border border-rose-200 font-mono">
          <TrendingDown className="w-3 h-3" />
          {(score * 100).toFixed(0)} Bearish
        </span>
      );
    }
    return (
      <span className="inline-flex items-center gap-1 text-[11px] font-semibold text-slate-600 bg-slate-50 px-2 py-0.5 rounded-full border border-slate-200 font-mono">
        Neutral
      </span>
    );
  };

  return (
    <Card className={`bg-white border-slate-200 shadow-sm flex flex-col h-full ${className || ""}`}>
      <CardHeader className="pb-3 border-b border-slate-100">
        <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3">
          <div className="flex items-center gap-2.5">
            <div className="w-8 h-8 rounded-lg bg-emerald-50 border border-emerald-200 flex items-center justify-center text-emerald-600">
              <Zap className="w-4 h-4 fill-emerald-600" />
            </div>
            <div>
              <CardTitle className="text-base font-bold text-slate-900 flex items-center gap-2">
                <span>Real-Time Event Feed</span>
                <span className="inline-flex items-center px-2 py-0.5 rounded-full text-[10px] font-mono font-medium bg-emerald-100 text-emerald-800">
                  Live Ingestion
                </span>
              </CardTitle>
              <p className="text-xs text-slate-500">
                Solana WebSockets, Pyth price drifts, and financial news signals
              </p>
            </div>
          </div>

          <Button
            variant="emerald"
            size="sm"
            onClick={handleReplayClick}
            disabled={isPlaying}
            className="flex items-center gap-1.5 font-bold shadow-sm text-xs self-start sm:self-auto"
          >
            <Play className="w-3.5 h-3.5 fill-current" />
            <span>{isPlaying ? "Replaying Demo..." : "Run Demo Replay"}</span>
          </Button>
        </div>

        {/* Filter and Search Bar */}
        <div className="flex flex-col sm:flex-row items-center gap-2 pt-3">
          <div className="relative flex-1 w-full">
            <Search className="w-3.5 h-3.5 absolute left-2.5 top-1/2 -translate-y-1/2 text-slate-400" />
            <Input
              type="text"
              placeholder="Search by symbol, headline, or source..."
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              className="pl-8 h-8 text-xs font-sans w-full"
            />
          </div>

          <div className="flex items-center gap-1 overflow-x-auto w-full sm:w-auto pb-1 sm:pb-0">
            {["ALL", "EARNINGS_BEAT", "DRIFT_REBALANCE", "DEPOSIT_DETECTED"].map((f) => (
              <button
                key={f}
                onClick={() => dispatch(setFilterType(f))}
                className={`px-2.5 py-1 rounded-md text-[10px] font-semibold whitespace-nowrap transition-colors ${
                  filterType === f
                    ? "bg-slate-900 text-white shadow-xs"
                    : "bg-slate-100 text-slate-600 hover:bg-slate-200"
                }`}
              >
                {f === "ALL" ? "All" : f.replace("_", " ")}
              </button>
            ))}
          </div>
        </div>
      </CardHeader>

      <CardContent className="p-3 flex-1 overflow-y-auto space-y-2.5 max-h-[600px]">
        {filteredEvents.length === 0 ? (
          <div className="p-8 text-center text-slate-400 text-xs">
            No events match your filter criteria.
          </div>
        ) : (
          filteredEvents.map((ev) => {
            const isSelected = ev.event_id === selectedEventId;
            const payload = ev.payload as any;
            const headline =
              payload?.headline ||
              `External trigger on ${payload?.symbol || "vault asset"}`;
            const symbol = payload?.symbol || (ev.vault_address ? "VAULT" : "SOL");

            return (
              <div
                key={ev.event_id}
                onClick={() => handleSelect(ev.event_id)}
                className={`p-3.5 rounded-xl border transition-all cursor-pointer text-left relative ${
                  isSelected
                    ? "bg-emerald-50/40 border-emerald-400 shadow-sm ring-1 ring-emerald-400/20"
                    : "bg-white border-slate-200 hover:border-slate-300 hover:bg-slate-50/60"
                }`}
              >
                {/* Header Row: Symbol, Badges, Time */}
                <div className="flex items-center justify-between gap-2 mb-1.5">
                  <div className="flex items-center gap-2">
                    <span className="font-mono font-extrabold text-xs text-slate-900 bg-slate-100 px-1.5 py-0.5 rounded border border-slate-200">
                      {symbol}
                    </span>
                    {getEventTypeBadge(ev.event_type)}
                    {getSentimentBadge(ev.sentiment_score)}
                  </div>

                  <div className="flex items-center gap-1 text-[10px] text-slate-400 font-mono">
                    <Clock className="w-3 h-3" />
                    <span>{new Date(ev.detected_at).toLocaleTimeString()}</span>
                  </div>
                </div>

                {/* Headline / Summary */}
                <h4 className="text-xs font-semibold text-slate-800 line-clamp-2 leading-relaxed">
                  {headline}
                </h4>

                {/* Footer Details: Source, Status */}
                <div className="flex items-center justify-between mt-2 pt-2 border-t border-slate-100/80 text-[10px] text-slate-500 font-mono">
                  <div className="flex items-center gap-1.5">
                    <Globe className="w-3 h-3 text-slate-400" />
                    <span className="capitalize">{ev.source.replace("_", " ")}</span>
                  </div>

                  <div className="flex items-center gap-1 font-semibold">
                    <span className="w-1.5 h-1.5 rounded-full bg-emerald-500" />
                    <span className="text-emerald-700">{ev.status}</span>
                  </div>
                </div>
              </div>
            );
          })
        )}
      </CardContent>
    </Card>
  );
}
