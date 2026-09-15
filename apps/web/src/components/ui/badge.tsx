import * as React from "react";
import { cva, type VariantProps } from "class-variance-authority";

import { cn } from "../../lib/utils";

const badgeVariants = cva(
  "inline-flex items-center rounded-md border px-2.5 py-0.5 text-xs font-semibold transition-colors focus:outline-none focus:ring-2 focus:ring-slate-950 focus:ring-offset-2",
  {
    variants: {
      variant: {
        default:
          "border-transparent bg-slate-900 text-slate-50 shadow hover:bg-slate-900/80",
        secondary:
          "border-transparent bg-slate-100 text-slate-900 hover:bg-slate-100/80",
        destructive:
          "border-transparent bg-rose-500 text-slate-50 shadow hover:bg-rose-500/80",
        outline: "text-slate-950 border-slate-200",
        success:
          "border-emerald-200 bg-emerald-50 text-emerald-700 font-medium",
        emerald:
          "border-emerald-200 bg-emerald-50 text-emerald-700 font-medium",
        purple:
          "border-purple-200 bg-purple-50 text-purple-700 font-medium",
        warning:
          "border-amber-200 bg-amber-50 text-amber-700 font-medium",
        cyan:
          "border-cyan-200 bg-cyan-50 text-cyan-700 font-medium",
        rose:
          "border-rose-200 bg-rose-50 text-rose-700 font-medium",
      },

    },
    defaultVariants: {
      variant: "default",
    },
  }
);

export interface BadgeProps
  extends React.HTMLAttributes<HTMLDivElement>,
    VariantProps<typeof badgeVariants> {}

function Badge({ className, variant, ...props }: BadgeProps) {
  return (
    <div className={cn(badgeVariants({ variant }), className)} {...props} />
  );
}

export { Badge, badgeVariants };
