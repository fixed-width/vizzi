#!/usr/bin/env python3
"""Replay the frozen review input through the finite HTTP/stdio diagnostic adapter."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import select
import subprocess
import sys
import tempfile
import time


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--fixture",type=Path,required=True)
    parser.add_argument("--output",type=Path,required=True)
    parser.add_argument("--binary",type=Path,default=Path("target/debug/vizzi"))
    args=parser.parse_args()
    here=Path(__file__).resolve().parent
    oracle=json.loads((here/"fixtures/oracle.json").read_text())
    manifest=json.loads((args.fixture/"manifest.json").read_text())
    workspace=(args.fixture/"workspace").resolve();output=args.output.resolve()
    if output==workspace or workspace in output.parents:raise ValueError("output must be outside fixture workspace")
    output.mkdir(parents=True,exist_ok=False)
    def verify():
        subprocess.run([sys.executable,str(here/"verify.py"),"--fixture",str(args.fixture)],check=True,stdout=subprocess.DEVNULL)
    verify()
    env={key:value for key,value in os.environ.items() if not key.startswith("GIT_")}
    env.update(GIT_OPTIONAL_LOCKS="0",GIT_CONFIG_NOSYSTEM="1",GIT_CONFIG_GLOBAL="/dev/null")
    base=subprocess.check_output(["git","-C",str(workspace),"show",f"{oracle['base_revision']}:{oracle['source_path']}"],env=env)
    operations=[];sequence=0
    with tempfile.TemporaryDirectory(prefix="vizzi-transport-cache-") as cache, (output/"host.log").open("w") as log:
        process=subprocess.Popen(["nice","-n","10",str(args.binary.resolve()),"--root",str(workspace),"--cache",cache,"--workers","1","--stdio"],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=log,text=True,bufsize=1)
        def request(method,uri,body=None,expected=200):
            nonlocal sequence
            sequence+=1;packet={"id":sequence,"method":method,"uri":uri}
            if body is not None:packet["body"]=body
            process.stdin.write(json.dumps(packet)+"\n");process.stdin.flush()
            if not select.select([process.stdout],[],[],60)[0]:raise TimeoutError(uri)
            line=process.stdout.readline()
            if not line:raise RuntimeError("host ended before response")
            result=json.loads(line)
            if result["id"]!=sequence or result["status"]!=expected:raise ValueError((uri,result))
            operations.append({"method":method,"uri":uri,"status":result["status"],"body_bytes":len(result["body"].encode())})
            return json.loads(result["body"]) if result["content_type"].startswith("application/json") else result["body"]
        def ready(ticket):
            deadline=time.monotonic()+60
            while time.monotonic()<deadline:
                status=request("GET","/api/comparisons/status")
                if status["ticket"]==ticket and not status["pending"]:
                    if status["error"] is not None:raise ValueError(status)
                    return status["active"]
                time.sleep(0.05)
            raise TimeoutError("capture status")
        def header(hunk):
            def side(value):
                start=value["lines"]["start"];count=value["lines"]["end"]-start
                return f"{start+1 if count else start},{count}"
            return f"@@ -{side(hunk['base'])} +{side(hunk['working'])} @@"
        def window(token,row,side,data):
            version=row[f"{side}_version"]
            body=request("GET",f"/api/comparisons/{token}/window?file={row['file']}&side={side}&version={version}&line_start=140&line_count=48")
            assert body["token"]==token and body["side"]==side and body["window"]["version"]==version
            lines=data.decode().split("\n")
            for line in body["window"]["lines"]:
                original=lines[line["number"]-1].removesuffix("\r").encode()
                assert line["text"]==original[line["column"]:line["end_column"]].decode()
            assert len(body["window"]["lines"])<=96 and sum(len(line["text"].encode()) for line in body["window"]["lines"])<=32768
            names=[entry["name"] for entry in body["window"]["outline"]]
            assert "compile_file" in names and "read_file" in names
            return {"side":side,"version":version,"line_rows":len(body["window"]["lines"]),"outline_total":body["window"]["outline_total"],"contexts":body["contexts"]}
        try:
            accepted=request("POST","/api/comparisons",{"provider":"git","revision":oracle["base_revision"]},202)
            metadata=ready(accepted["ticket"]);token=metadata["token"];assert metadata["resolved"]==oracle["base_revision"]
            files=request("GET",f"/api/comparisons/{token}/files")
            assert files["token"]==token and files["total"]==2 and files["next"] is None
            assert sorted(row["path"] for row in files["files"])==sorted(oracle["changed_paths"])
            row=next(row for row in files["files"] if row["path"]==oracle["source_path"])
            diff=request("GET",f"/api/comparisons/{token}/diff?file={row['file']}&base_version={row['base_version']}&working_version={row['working_version']}")
            assert [header(hunk) for hunk in diff["hunks"]]==oracle["variants"][manifest["variant"]]["base_to_working_hunks"]
            windows=[window(token,row,"base",base),window(token,row,"working",(workspace/oracle["source_path"]).read_bytes())]
            document=next(row for row in files["files"] if row["path"]==oracle["untracked_path"])
            added=request("GET",f"/api/comparisons/{token}/diff?file={document['file']}&working_version={document['working_version']}")
            assert [header(hunk) for hunk in added["hunks"]]==["@@ -0,0 +1,5 @@"]
            request("GET",f"/api/comparisons/{token}/window?file={row['file']}&side=working&version=wrong",expected=409)
            refreshed=request("POST",f"/api/comparisons/{token}/refresh",{},202)
            new=ready(refreshed["ticket"]);assert new["token"]!=token and new["resolved"]==metadata["resolved"] and new["comparison"]==metadata["comparison"]
            request("GET",f"/api/comparisons/{token}/files",expected=409)
            request("DELETE",f"/api/comparisons/{new['token']}",{},204)
            request("GET",f"/api/comparisons/{new['token']}/files",expected=409)
            request("GET","/api/comparisons/events",expected=400)
        finally:
            process.stdin.close()
            try:process.wait(timeout=10)
            except subprocess.TimeoutExpired:process.kill();process.wait();raise
        assert process.returncode==0,process.returncode
    verify()
    report={"schema":1,"variant":manifest["variant"],"base_revision":oracle["base_revision"],"verified":True,"metadata":metadata,"refreshed":new,"totals":files["totals"],"hunk_headers":[header(hunk) for hunk in diff["hunks"]],"windows":windows,"operations":operations,"fixture_reverified":True,"oracle_sha256":hashlib.sha256((here/"fixtures/oracle.json").read_bytes()).hexdigest(),"scope":"Finite diagnostic requests execute the same HTTP handlers; no browser/UI or human study."}
    (output/"transport.json").write_text(json.dumps(report,indent=2)+"\n")
    print(f"{manifest['variant']}: transport inventory, source, hunks, refresh and expiry verified")


if __name__=="__main__":main()
