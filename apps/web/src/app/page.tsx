"use client";

import React from "react";
import Link from "next/link";
import { ArrowRight, ShieldCheck, TrendingUp, Layers } from "lucide-react";

import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";

export default function HomePage() {
  return (
    <div className="pt-8 pb-16 space-y-16">
      {/* Hero Section */}
      <div className="max-w-3xl mx-auto text-center space-y-6">
        <span className="inline-flex items-center gap-1.5 px-3 py-1 rounded-full text-xs font-semibold bg-emerald-50 text-emerald-700 border border-emerald-200">
          <ShieldCheck className="w-3.5 h-3.5 text-emerald-600" />
          <span>Simple, Protected Equity Vaults</span>
        </span>

        <h1 className="text-4xl sm:text-5xl font-extrabold tracking-tight text-slate-900 leading-tight">
          Automated Asset Management for Everyone
        </h1>

        <p className="text-base sm:text-lg text-slate-600 max-w-xl mx-auto leading-relaxed">
          Monitor your portfolio, trade verified equities, and rebalance automatically with transparent safety guardrails.
        </p>

        {/* CTA Buttons */}
        <div className="flex flex-col sm:flex-row items-center justify-center gap-4 pt-2">
          <Link href="/dashboard">
            <Button size="lg" className="w-full sm:w-auto font-semibold px-8 bg-emerald-600 hover:bg-emerald-700 text-white shadow-sm">
              <span>Open Dashboard</span>
              <ArrowRight className="w-4 h-4 ml-2" />
            </Button>
          </Link>

          <Link href="/markets">
            <Button variant="outline" size="lg" className="w-full sm:w-auto font-semibold px-8 border-slate-300 text-slate-700 hover:bg-slate-50">
              <span>Explore Markets</span>
            </Button>
          </Link>
        </div>
      </div>

      {/* Pillar Cards */}
      <div className="grid grid-cols-1 md:grid-cols-3 gap-6 max-w-5xl mx-auto">
        {/* Pillar 1 */}
        <Card className="bg-white border-slate-200 shadow-sm rounded-xl">
          <CardContent className="p-6 space-y-2.5">
            <div className="w-10 h-10 rounded-lg bg-emerald-50 border border-emerald-200 flex items-center justify-center text-emerald-700">
              <ShieldCheck className="w-5 h-5" />
            </div>
            <h3 className="text-base font-bold text-slate-900">Protected by Default</h3>
            <p className="text-xs text-slate-600 leading-relaxed">
              Every rebalance is automatically bounded by cash reserve buffers and position exposure caps to protect capital.
            </p>
          </CardContent>
        </Card>

        {/* Pillar 2 */}
        <Card className="bg-white border-slate-200 shadow-sm rounded-xl">
          <CardContent className="p-6 space-y-2.5">
            <div className="w-10 h-10 rounded-lg bg-emerald-50 border border-emerald-200 flex items-center justify-center text-emerald-700">
              <TrendingUp className="w-5 h-5" />
            </div>
            <h3 className="text-base font-bold text-slate-900">Live Verified Pricing</h3>
            <p className="text-xs text-slate-600 leading-relaxed">
              All portfolio valuations are derived directly from verified oracle price streams without simulated delays.
            </p>
          </CardContent>
        </Card>

        {/* Pillar 3 */}
        <Card className="bg-white border-slate-200 shadow-sm rounded-xl">
          <CardContent className="p-6 space-y-2.5">
            <div className="w-10 h-10 rounded-lg bg-emerald-50 border border-emerald-200 flex items-center justify-center text-emerald-700">
              <Layers className="w-5 h-5" />
            </div>
            <h3 className="text-base font-bold text-slate-900">Seamless Rebalancing</h3>
            <p className="text-xs text-slate-600 leading-relaxed">
              One-click portfolio rebalancing automatically realigns your holdings to your target allocation.
            </p>
          </CardContent>
        </Card>
      </div>
    </div>
  );
}
