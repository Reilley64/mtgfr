import { cn } from "~/lib/cn";

/** Primary act emphasis — larger silhouette + glow only while this seat must act. */
export function priorityPrimaryClass(yours: boolean): string {
  return cn(yours && "min-w-[156px] px-8 py-[13px] text-[16px] leading-none shadow-glow");
}
