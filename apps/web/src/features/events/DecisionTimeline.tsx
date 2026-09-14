"use client";

import React, { useEffect } from "react";
import {
  AlertTriangle,
  ArrowDown,
  ArrowRight,
  Check,
  CheckCircle2,
  ChevronRight,
  Clock,
  Coins,
  Cpu,
  ExternalLink,
  FastForward,
  FileCheck2,
  Pause,
  Play,
  RefreshCw,
  RotateCcw,
  ShieldAlert,
  ShieldCheck,
  Sparkles,
  TrendingUp,
  Zap,
} from "lucide-react";
import { useAppDispatch, useAppSelector } from "../../store/hooks";
import {
  setActiveStage,
  nextStage,
  prevStage,
  setIsPlaying,
  startReplay,
} from "../../store/eventsSlice";
import { Card, CardContent, CardHeader, CardTitle } from "../../components/ui/card";
import { Badge } from "../../components/ui/badge";
import { Button } from "../../components/ui/button";

const PIPELINE_STEPS = [
  {
    step: 1,
    title: "Event detected",
    icon: Zap,
    tag: "Market Ingestion",
    color: "emerald",
  },
  {
    step: 2,
    title: "Policy evaluated",
    icon: Cpu,
    tag: "Rule & Signal Match",
    color: "cyan",
  },
  {
    step: 3,
    title: "Risk evaluated",
    icon: ShieldCheck,
    tag: "4-Factor Risk Guard",
    color: "purple",
  },
  {
    step: 4,
    title: "Decision generated",
    icon: FileCheck2,
    tag: "Order Synthesis",
    color: "blue",
  },
  {
    step: 5,
    title: "Execution submitted",
    icon: Coins,
    tag: "Jupiter DEX Routing",
    color: "amber",
  },
  {
    step: 6,
    title: "Transaction confirmed",
    icon: CheckCircle2,
    tag: "Solana Devnet Finalized",
    color: "emerald",
  },
];

interface DecisionTimelineProps {
  className?: string;
}

export function DecisionTimeline({ className }: DecisionTimelineProps) {
  const dispatch = useAppDispatch();
  const { selectedEventId, activeStage, isPlaying, pipelines } = useAppSelector(
    (state) => state.events
  );

  // Load pipeline details for the selected event (or fallback to default pipeline)
  const pipeline =
    pipelines[selectedEventId] ||
    pipelines["a8b1c2d3-e4f5-4678-90ab-cdef12345678"];

  // Automated playback ticker
  useEffect(() => {
    let timer: NodeJS.Timeout;
    if (isPlaying) {
      timer = setInterval(() => {
        dispatch(nextStage());
      }, 1200);
    }
    return () => {
      if (timer) clearInterval(timer);
    };
  }, [isPlaying, dispatch]);

  const handleStepClick = (stepNum: number) => {
    dispatch(setIsPlaying(false));
    dispatch(setActiveStage(stepNum));
  };

  return (
    <Card className={`bg-white border-slate-200 shadow-sm flex flex-col ${className || ""}`}>
      {/* Header with Pipeline Title & Playback Controls */}
      <CardHeader className="pb-4 border-b border-slate-100">
        <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-3">
          <div>
            <div className="flex items-center gap-2">
              <Badge variant="cyan" className="font-mono text-[10px] font-bold">
                Backend Pipeline Visualizer
              </Badge>
              <span className="text-xs text-slate-400 font-mono">•</span>
              <span className="text-xs text-slate-500 font-mono">
                Step {activeStage} of 6
              </span>
            </div>
            <CardTitle className="text-lg font-extrabold text-slate-900 mt-1 flex items-center gap-2">
              <span>Decision Lifecycle Timeline</span>
            </CardTitle>
            <p className="text-xs text-slate-500">
              End-to-end execution audit from external market signal to Solana on-chain finality
            </p>
          </div>

          {/* Controls: Play, Step Back, Step Forward, Reset */}
          <div className="flex items-center gap-1.5 self-start sm:self-auto bg-slate-50 p-1.5 rounded-xl border border-slate-200">
            <Button
              variant="outline"
              size="sm"
              onClick={() => dispatch(prevStage())}
              disabled={activeStage <= 1 || isPlaying}
              className="h-8 px-2 text-xs font-semibold"
              title="Step backward"
            >
              Prev
            </Button>

            <Button
              variant={isPlaying ? "destructive" : "emerald"}
              size="sm"
              onClick={() => {
                if (isPlaying) {
                  dispatch(setIsPlaying(false));
                } else {
                  if (activeStage === 6) {
                    dispatch(startReplay());
                  } else {
                    dispatch(setIsPlaying(true));
                  }
                }
              }}
              className="h-8 px-3 font-bold text-xs flex items-center gap-1.5"
            >
              {isPlaying ? (
                <>
                  <Pause className="w-3.5 h-3.5 fill-current" />
                  <span>Pause</span>
                </>
              ) : (
                <>
                  <Play className="w-3.5 h-3.5 fill-current" />
                  <span>{activeStage === 6 ? "Replay Flow" : "Play"}</span>
                </>
              )}
            </Button>

            <Button
              variant="outline"
              size="sm"
              onClick={() => dispatch(nextStage())}
              disabled={activeStage >= 6 || isPlaying}
              className="h-8 px-2 text-xs font-semibold"
              title="Step forward"
            >
              Next
            </Button>

            <Button
              variant="ghost"
              size="icon"
              onClick={() => {
                dispatch(setIsPlaying(false));
                dispatch(setActiveStage(6));
              }}
              className="h-8 w-8 text-slate-500 hover:text-slate-900"
              title="Jump to complete"
            >
              <FastForward className="w-4 h-4" />
            </Button>
          </div>
        </div>

        {/* Step Progression Tabs (Horizontal Bar) */}
        <div className="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-6 gap-1.5 pt-4">
          {PIPELINE_STEPS.map((s) => {
            const isCompleted = activeStage > s.step;
            const isCurrent = activeStage === s.step;
            const Icon = s.icon;

            return (
              <button
                key={s.step}
                onClick={() => handleStepClick(s.step)}
                className={`flex items-center gap-2 p-2 rounded-lg border text-left transition-all ${
                  isCurrent
                    ? "bg-slate-900 text-white border-slate-900 shadow-sm"
                    : isCompleted
                    ? "bg-emerald-50/70 border-emerald-200 text-emerald-800 hover:bg-emerald-100/60"
                    : "bg-slate-50 border-slate-200/80 text-slate-400 hover:bg-slate-100"
                }`}
              >
                <div
                  className={`w-6 h-6 rounded-md flex items-center justify-center shrink-0 text-xs font-bold font-mono ${
                    isCurrent
                      ? "bg-emerald-500 text-slate-950"
                      : isCompleted
                      ? "bg-emerald-600 text-white"
                      : "bg-slate-200 text-slate-600"
                  }`}
                >
                  {isCompleted ? <Check className="w-3.5 h-3.5 stroke-[3]" /> : s.step}
                </div>

                <div className="min-w-0">
                  <div className="text-[11px] font-bold truncate leading-tight">
                    {s.title}
                  </div>
                  <div
                    className={`text-[9px] font-mono truncate ${
                      isCurrent ? "text-slate-300" : isCompleted ? "text-emerald-700" : "text-slate-400"
                    }`}
                  >
                    {s.tag}
                  </div>
                </div>
              </button>
            );
          })}
        </div>
      </CardHeader>

      {/* Main Timeline Body */}
      <CardContent className="p-6 space-y-6">
        {PIPELINE_STEPS.map((stepMeta, index) => {
          const stageNumber = stepMeta.step;
          const isReached = activeStage >= stageNumber;
          const isCurrent = activeStage === stageNumber;
          const Icon = stepMeta.icon;
          const stageDetails = pipeline.stages[stageNumber - 1];

          return (
            <div key={stageNumber} className="relative">
              {/* Connector line and downward arrow to next step */}
              {index < PIPELINE_STEPS.length - 1 && (
                <div
                  className={`absolute left-5 top-12 bottom-0 w-0.5 -ml-px transition-colors ${
                    activeStage > stageNumber ? "bg-emerald-500" : "bg-slate-200"
                  }`}
                >
                  {isCurrent && (
                    <div className="absolute top-1/2 -left-1.5 w-3.5 h-3.5 rounded-full bg-emerald-500 animate-ping opacity-75" />
                  )}
                </div>
              )}

              {/* Step Card Row */}
              <div
                className={`flex items-start gap-4 p-4 rounded-xl border transition-all ${
                  isCurrent
                    ? "bg-gradient-to-r from-emerald-50/60 via-slate-50 to-white border-emerald-400 shadow-md ring-1 ring-emerald-400/30"
                    : isReached
                    ? "bg-white border-slate-200 hover:border-slate-300 shadow-xs"
                    : "bg-slate-50/50 border-slate-200/60 opacity-60"
                }`}
              >
                {/* Step Circle Indicator */}
                <div
                  className={`w-10 h-10 rounded-xl flex items-center justify-center shrink-0 transition-all ${
                    isCurrent
                      ? "bg-slate-900 text-emerald-400 shadow-md scale-105"
                      : isReached
                      ? "bg-emerald-500 text-white"
                      : "bg-slate-100 text-slate-400 border border-slate-200"
                  }`}
                >
                  <Icon className="w-5 h-5" />
                </div>

                {/* Stage Content */}
                <div className="flex-1 space-y-3">
                  <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-1.5">
                    <div>
                      <div className="flex items-center gap-2">
                        <span className="text-xs font-mono font-bold text-slate-400">
                          STAGE {stageNumber}
                        </span>
                        <h3 className="text-base font-extrabold text-slate-900">
                          {stepMeta.title}
                        </h3>
                        {isCurrent && (
                          <Badge variant="success" className="animate-pulse text-[10px] font-mono">
                            Active Evaluation
                          </Badge>
                        )}
                      </div>
                      <p className="text-xs text-slate-500 font-medium">
                        {stageDetails?.subtitle}
                      </p>
                    </div>

                    <div className="flex items-center gap-2 text-[11px] font-mono text-slate-400">
                      <Clock className="w-3.5 h-3.5 text-slate-400" />
                      <span>{stageDetails?.timestamp}</span>
                    </div>
                  </div>

                  {/* Stage Narrative Description */}
                  <p className="text-xs text-slate-700 leading-relaxed bg-slate-50/80 p-2.5 rounded-lg border border-slate-100">
                    {stageDetails?.description}
                  </p>

                  {/* Metrics & Parameters Breakdown Grid */}
                  {isReached && stageDetails?.metrics && (
                    <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-2.5 pt-1">
                      {Object.entries(stageDetails.metrics).map(([key, val]) => (
                        <div
                          key={key}
                          className="p-2 rounded-lg bg-white border border-slate-200/90 shadow-xs flex flex-col justify-between"
                        >
                          <span className="text-[10px] uppercase font-bold text-slate-400 font-mono tracking-wider">
                            {key}
                          </span>
                          <span className="text-xs font-bold text-slate-900 font-mono mt-0.5 truncate">
                            {String(val)}
                          </span>
                        </div>
                      ))}
                    </div>
                  )}
                </div>
              </div>

              {/* Downward connecting indicator between steps */}
              {index < PIPELINE_STEPS.length - 1 && (
                <div className="flex items-center justify-center my-1.5">
                  <div
                    className={`w-6 h-6 rounded-full flex items-center justify-center transition-colors ${
                      activeStage > stageNumber
                        ? "bg-emerald-100 text-emerald-700"
                        : "bg-slate-100 text-slate-300"
                    }`}
                  >
                    <ArrowDown className="w-3.5 h-3.5 stroke-[2.5]" />
                  </div>
                </div>
              )}
            </div>
          );
        })}
      </CardContent>
    </Card>
  );
}
