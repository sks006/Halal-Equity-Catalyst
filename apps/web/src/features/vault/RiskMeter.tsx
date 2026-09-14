"use client";

import React from "react";
import { AlertTriangle, CheckCircle2, Shield } from "lucide-react";

import { Card, CardContent, CardHeader, CardTitle } from "../../components/ui/card";
import { Badge } from "../../components/ui/badge";
import { Progress } from "../../components/ui/progress";

interface Props {
  currentLtvBps: number;
  maxLtvBps: number;
  currentPositionBps: number;
  maxPositionBps: number;
}

export function RiskMeter({
  currentLtvBps,
  maxLtvBps,
  currentPositionBps,
  maxPositionBps,
}: Props) {
  const ltvUsagePct = maxLtvBps > 0 ? (currentLtvBps / maxLtvBps) * 100 : 0;
  const positionUsagePct = maxPositionBps > 0 ? (currentPositionBps / maxPositionBps) * 100 : 0;

  const isLtvHigh = ltvUsagePct > 80;
  const isPosHigh = positionUsagePct > 85;

  return (
    <Card className="bg-white">
      <CardHeader className="p-5 pb-3">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <div className="w-8 h-8 rounded-lg bg-emerald-50 flex items-center justify-center text-emerald-600 border border-emerald-100">
              <Shield className="w-4 h-4" />
            </div>
            <div>
              <CardTitle className="text-sm font-bold text-slate-900">
                On-Chain Risk Defense Line
              </CardTitle>
              <p className="text-xs text-slate-500">Autonomous risk constraints verified before execution</p>
            </div>
          </div>

          {isLtvHigh || isPosHigh ? (
            <Badge variant="warning" className="flex items-center gap-1 font-medium">
              <AlertTriangle className="w-3 h-3" />
              <span>Limit Warning</span>
            </Badge>
          ) : (
            <Badge variant="success" className="flex items-center gap-1 font-medium">
              <CheckCircle2 className="w-3 h-3" />
              <span>Within Policy</span>
            </Badge>
          )}
        </div>
      </CardHeader>

      <CardContent className="p-5 pt-2 space-y-4">
        {/* LTV Meter */}
        <div className="space-y-1.5">
          <div className="flex justify-between text-xs font-mono">
            <span className="text-slate-600 font-medium">Loan-to-Value (LTV) Usage</span>
            <span className="text-slate-900 font-bold">
              {(currentLtvBps / 100).toFixed(1)}% / {(maxLtvBps / 100).toFixed(1)}% Max
            </span>
          </div>
          <Progress
            value={Math.min(ltvUsagePct, 100)}
            indicatorClassName={isLtvHigh ? "bg-rose-500" : ltvUsagePct > 60 ? "bg-amber-500" : "bg-emerald-600"}
          />
        </div>

        {/* Single Position Exposure Meter */}
        <div className="space-y-1.5">
          <div className="flex justify-between text-xs font-mono">
            <span className="text-slate-600 font-medium">Max Single Position Exposure</span>
            <span className="text-slate-900 font-bold">
              {(currentPositionBps / 100).toFixed(1)}% / {(maxPositionBps / 100).toFixed(1)}% Max
            </span>
          </div>
          <Progress
            value={Math.min(positionUsagePct, 100)}
            indicatorClassName={isPosHigh ? "bg-rose-500" : positionUsagePct > 70 ? "bg-amber-500" : "bg-cyan-600"}
          />
        </div>

        <div className="pt-3 border-t border-slate-100 flex items-center justify-between text-xs text-slate-500 font-mono">
          <span>Dual Line Defense:</span>
          <span className="text-emerald-700 font-semibold">Anchor CPI Program Enforced</span>
        </div>
      </CardContent>
    </Card>
  );
}
