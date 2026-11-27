"""Figures of P4 question 13, owner decision of 2026-11-27, steps 1-2 (from out/p4/q13v4/{cre,thresh_s1}.json):
docs/figures/P4/q13_v4_step1.png and q13_v4_step2.png.  python tools/q13_v4_figs.py"""
import json, numpy as np, matplotlib
matplotlib.use("Agg"); import matplotlib.pyplot as plt
R="/home/blue-whale/Projects/ToyClimateModel/"
c=json.load(open(R+"out/p4/q13v4/cre.json"))
fig,ax=plt.subplots(1,3,figsize=(15,5.2))
col={"s1":"#1f77b4","v2":"#ff7f0e","v4":"#2ca02c"}
for lab in ("s1","v2","v4"):
    lv=[e for e in c[lab]["levels"] if e["sigma"]>0.6]
    sg=np.array([e["sigma"] for e in lv]); vol=np.array([e["vol"] for e in lv])
    sh=json.load(open(R+{"s1":"out/p4/q13scr/s1_ref","v2":"out/p4/q13res/v2_l30","v4":"out/p4/q13res/v4_l32"}[lab]+"/meta.json"))["config"]["sigma_half"]
    sh=np.array(sh); dsg=np.diff(sh)[-len(sg):]
    ax[0].plot(np.array([e["cl"] for e in lv]),sg*1000,"o-",color=col[lab],label=f"{lab} total")
    ax[0].plot(np.array([e["cs"] for e in lv]),sg*1000,"s--",color=col[lab],alpha=.6,label=f"{lab} stratiform")
    ax[1].plot(np.array([e["wpath"] for e in lv])/(dsg*1000),sg*1000,"o-",color=col[lab],label=lab)
for a in ax[:2]: a.invert_yaxis(); a.set_ylabel("σ × 1000"); a.grid(alpha=.3)
ax[0].set_xlabel("cloud fraction, 20°S–20°N"); ax[0].legend(fontsize=7)
ax[1].set_xlabel("grid-mean cloud water per 10 hPa of layer [g m⁻²]"); ax[1].legend()
ax[0].set_title("cloud by level"); ax[1].set_title("water per unit layer depth")
labs=["s1","s1 with\nL32 water\n(×½)","v4 with\nL26 water\n(×2)","v4","v2"]
vals=[c["s1"]["all"]["net"],c["s1"]["other_water_all"]["net"],c["v4"]["other_water_all"]["net"],c["v4"]["all"]["net"],c["v2"]["all"]["net"]]
ax[2].bar(range(5),vals,color=["#1f77b4","#9ecae1","#98df8a","#2ca02c","#ff7f0e"])
for i,v in enumerate(vals): ax[2].text(i,v-1.5,f"{v:.1f}",ha="center")
ax[2].axhline(-14.6-5.1,ls=":",color="k"); ax[2].text(4.4,-14.6-5.1+0.6,"CERES −14.6\n(+ offline bias −5)",ha="right",fontsize=8)
ax[2].set_xticks(range(5)); ax[2].set_xticklabels(labs,fontsize=8); ax[2].set_ylabel("net CRE 20°S–20°N, offline [W m⁻²]")
ax[2].set_title("arithmetic test"); ax[2].grid(alpha=.3,axis="y")
fig.suptitle("P4 Q13 step 1: why L32 (v4) dims the tropics (T21, year 2)"); fig.tight_layout()
fig.savefig(R+"docs/figures/P4/q13_v4_step1.png",dpi=110)

t=json.load(open(R+"out/p4/q13v4/thresh_s1.json"))
fig,ax=plt.subplots(1,2,figsize=(13,5))
xs=["0.75","0.80","0.85","0.90","0.95"]; xv=[float(x) for x in xs]
sty={"model":"#1f77b4","model shifted to NCEP mean":"#ff7f0e","NCEP daily 2015–16":"#d62728","NCEP mean profile":"#7f7f7f"}
for a,(reg,r) in zip(ax,t.items()):
    for src,cc in sty.items():
        for fd,gate,ls in (("on","on","-"),("off","off","--")):
            a.plot(xv,[r["grid"][f"{src}|{fd}|{gate}|{x}"][1] for x in xs],ls,color=cc,marker="o",ms=3,
                   label=f"{src}, fd {fd}, gate {gate}")
    a.axhline(r["goccp_low"],color="k",lw=2); a.text(0.755,r["goccp_low"]+0.008,"GOCCP low, ocean")
    a.axhline(r["run_low_cover"],color="k",ls=":"); a.text(0.755,r["run_low_cover"]+0.008,"s1 run")
    a.set_title(reg+" (s1, ocean)"); a.set_xlabel("rh_crit_surface"); a.set_ylabel("low cover (p > 680 hPa)"); a.grid(alpha=.3); a.set_ylim(0.1,0.65)
ax[0].legend(fontsize=7,loc="upper right")
fig.suptitle("P4 Q13 step 2: low cover the scheme gives, model against observed humidity"); fig.tight_layout()
fig.savefig(R+"docs/figures/P4/q13_v4_step2.png",dpi=110)
