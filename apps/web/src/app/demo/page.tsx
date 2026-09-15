"use client";

import React, { useState } from "react";
import Link from "next/link";
import {
  Activity,
  ArrowLeft,
  Code2,
  Database,
  ExternalLink,
  Layers,
  Play,
  RotateCcw,
  ShieldCheck,
  Sparkles,
  Terminal,
  Zap,
} from "lucide-react";
import { EventFeed } from "../../features/events/EventFeed";
import { DecisionTimeline } from "../../features/events/DecisionTimeline";
import { useAppDispatch, useAppSelector } from "../../store/hooks";
import { startReplay, setActiveStage, setIsPlaying } from "../../store/eventsSlice";
import { Card, CardContent, CardHeader, CardTitle } from "../../components/ui/card";
import { Button } from "../../components/ui/button";
import { Badge } from "../../components/ui/badge";
import { Tabs, TabsList, TabsTrigger, TabsContent } from "../../components/ui/tabs";

// Fixture JSON previews for the developer drawer
import nvdaEventJson from "../../../../../demo-data/nvda-earnings-event.json";
import sampleVaultJson from "../../../../../demo-data/sample-vault.json";
import samplePolicyJson from "../../../../../demo-data/sample-policy.json";
import samplePortfolioJson from "../../../../../demo-data/sample-portfolio.json";
import samplePositionJson from "../../../../../demo-data/sample-position.json";
import demoScriptJson from "../../../../../demo-data/demo-script.json";

export default function DemoPage() {
  const dispatch = useAppDispatch();
  const { isPlaying, activeStage } = useAppSelector((state) => state.events);
  const [selectedFixtureTab, setSelectedFixtureTab] = useState("script");

  const handleStartReplay = () => {
    dispatch(startReplay());
  };

  const handleReset = () => {
    dispatch(setIsPlaying(false));
    dispatch(setActiveStage(6));
  };

  return (
    <div className="space-y-8 pb-16">
      {/* Top Banner & Controls */}
      <div className="flex flex-col lg:flex-row lg:items-center justify-between gap-4 pb-6 border-b border-slate-200">
        <div>
          <div className="flex items-center gap-2 mb-1.5">
            <Link href="/dashboard">
              <Button variant="outline" size="icon" className="h-8 w-8 mr-1">
                <ArrowLeft className="w-4 h-4" />
              </Button>
            </Link>
            <Badge variant="success" className="gap-1 font-mono text-xs">
              <Sparkles className="w-3.5 h-3.5 text-emerald-600" />
              <span>Phase 14 & 15 Demo Mode</span>
            </Badge>
            <span className="text-slate-400 font-mono text-xs">•</span>
            <span className="text-xs text-slate-500 font-mono">Deterministic Engine Replay</span>
          </div>

          <h1 className="text-2xl sm:text-3xl font-extrabold text-slate-900 tracking-tight">
            Event & Decision Pipeline Visualization
          </h1>
          <p className="text-xs sm:text-sm text-slate-600 max-w-2xl mt-1">
            Visualizing the end-to-end autonomous pipeline: from external news ingestion to policy rule matching,
            4-factor risk assessment, Jupiter swap route quote, and Solana Anchor settlement.
          </p>
        </div>

        {/* Global Action Bar */}
        <div className="flex items-center gap-3 self-start lg:self-auto">
          <Button
            variant="outline"
            size="sm"
            onClick={handleReset}
            className="flex items-center gap-1.5 font-semibold text-xs h-9"
          >
            <RotateCcw className="w-3.5 h-3.5" />
            <span>Reset View</span>
          </Button>

          <Button
            variant="emerald"
            size="sm"
            onClick={handleStartReplay}
            disabled={isPlaying}
            className="flex items-center gap-2 font-bold text-xs h-9 shadow-sm"
          >
            <Play className="w-3.5 h-3.5 fill-current" />
            <span>{isPlaying ? "Replaying Pipeline..." : "Launch Replay Demo"}</span>
          </Button>
        </div>
      </div>

      {/* Protocol Live Status Grid */}
      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
        <Card className="bg-white border-slate-200 shadow-xs">
          <CardContent className="p-4">
            <span className="text-[11px] font-bold text-slate-400 uppercase tracking-wider font-mono">
              Scenario Under Test
            </span>
            <div className="mt-1 text-base font-extrabold text-slate-900 flex items-center gap-1.5">
              <Zap className="w-4 h-4 text-emerald-600 fill-emerald-600" />
              <span>NVDA Earnings Beat</span>
            </div>
            <span className="text-xs text-emerald-700 font-mono mt-0.5 block">
              +0.85 Bullish Sentiment Score
            </span>
          </CardContent>
        </Card>

        <Card className="bg-white border-slate-200 shadow-xs">
          <CardContent className="p-4">
            <span className="text-[11px] font-bold text-slate-400 uppercase tracking-wider font-mono">
              Target Smart Vault
            </span>
            <div className="mt-1 text-base font-extrabold text-slate-900 truncate">
              Solana Liquid Growth (SLGA)
            </div>
            <span className="text-xs text-slate-500 font-mono mt-0.5 block">
              $4,850,000 Total Collateral
            </span>
          </CardContent>
        </Card>

        <Card className="bg-white border-slate-200 shadow-xs">
          <CardContent className="p-4">
            <span className="text-[11px] font-bold text-slate-400 uppercase tracking-wider font-mono">
              Risk Defense Mode
            </span>
            <div className="mt-1 text-base font-extrabold text-slate-900 flex items-center gap-1.5">
              <ShieldCheck className="w-4 h-4 text-emerald-600" />
              <span>Dual-Layer Guard</span>
            </div>
            <span className="text-xs text-slate-500 font-mono mt-0.5 block">
              Rust Engine + Anchor CPI
            </span>
          </CardContent>
        </Card>

        <Card className="bg-white border-slate-200 shadow-xs">
          <CardContent className="p-4">
            <span className="text-[11px] font-bold text-slate-400 uppercase tracking-wider font-mono">
              Deterministic Script
            </span>
            <div className="mt-1 text-base font-extrabold text-slate-900 flex items-center gap-1.5 font-mono">
              <Terminal className="w-4 h-4 text-cyan-600" />
              <span>pnpm tsx replay-demo</span>
            </div>
            <span className="text-xs text-cyan-700 font-mono mt-0.5 block">
              Verified CLI Replay
            </span>
          </CardContent>
        </Card>
      </div>

      {/* Main Interactive Grid: EventFeed (Left) + DecisionTimeline (Right) */}
      <div className="grid grid-cols-1 lg:grid-cols-12 gap-6">
        {/* Left Column: Event Feed */}
        <div className="lg:col-span-5 flex flex-col">
          <EventFeed />
        </div>

        {/* Right Column: Decision Timeline */}
        <div className="lg:col-span-7 flex flex-col">
          <DecisionTimeline />
        </div>
      </div>

      {/* Bottom Section: Deterministic Fixtures & Data Inspector */}
      <Card className="bg-white border-slate-200 shadow-sm">
        <CardHeader className="pb-3 border-b border-slate-100">
          <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-2">
            <div className="flex items-center gap-2">
              <Database className="w-4 h-4 text-slate-700" />
              <CardTitle className="text-sm font-bold text-slate-900">
                Deterministic Demo Fixtures Inspector (demo-data/)
              </CardTitle>
            </div>
            <span className="text-xs text-slate-400 font-mono">
              Structured JSON payloads used for deterministic replay
            </span>
          </div>
        </CardHeader>

        <CardContent className="p-4">
          <Tabs
            defaultValue="script"
            value={selectedFixtureTab}
            onValueChange={setSelectedFixtureTab}
          >
            <TabsList className="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-6 mb-4">
              <TabsTrigger value="script" className="text-xs font-mono">
                demo-script
              </TabsTrigger>
              <TabsTrigger value="event" className="text-xs font-mono">
                nvda-event
              </TabsTrigger>
              <TabsTrigger value="vault" className="text-xs font-mono">
                sample-vault
              </TabsTrigger>
              <TabsTrigger value="policy" className="text-xs font-mono">
                sample-policy
              </TabsTrigger>
              <TabsTrigger value="portfolio" className="text-xs font-mono">
                sample-portfolio
              </TabsTrigger>
              <TabsTrigger value="position" className="text-xs font-mono">
                sample-position
              </TabsTrigger>
            </TabsList>

            <TabsContent value="script">
              <div className="rounded-lg bg-slate-950 p-4 font-mono text-xs text-emerald-400 overflow-x-auto max-h-96">
                <pre>{JSON.stringify(demoScriptJson, null, 2)}</pre>
              </div>
            </TabsContent>

            <TabsContent value="event">
              <div className="rounded-lg bg-slate-950 p-4 font-mono text-xs text-cyan-400 overflow-x-auto max-h-96">
                <pre>{JSON.stringify(nvdaEventJson, null, 2)}</pre>
              </div>
            </TabsContent>

            <TabsContent value="vault">
              <div className="rounded-lg bg-slate-950 p-4 font-mono text-xs text-amber-400 overflow-x-auto max-h-96">
                <pre>{JSON.stringify(sampleVaultJson, null, 2)}</pre>
              </div>
            </TabsContent>

            <TabsContent value="policy">
              <div className="rounded-lg bg-slate-950 p-4 font-mono text-xs text-purple-400 overflow-x-auto max-h-96">
                <pre>{JSON.stringify(samplePolicyJson, null, 2)}</pre>
              </div>
            </TabsContent>

            <TabsContent value="portfolio">
              <div className="rounded-lg bg-slate-950 p-4 font-mono text-xs text-slate-200 overflow-x-auto max-h-96">
                <pre>{JSON.stringify(samplePortfolioJson, null, 2)}</pre>
              </div>
            </TabsContent>

            <TabsContent value="position">
              <div className="rounded-lg bg-slate-950 p-4 font-mono text-xs text-rose-400 overflow-x-auto max-h-96">
                <pre>{JSON.stringify(samplePositionJson, null, 2)}</pre>
              </div>
            </TabsContent>
          </Tabs>
        </CardContent>
      </Card>
    </div>
  );
}
