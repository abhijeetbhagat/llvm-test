; ModuleID = 'mod1'

@main1 = private unnamed_addr constant [5 x i8] c"abhi\00"

declare i32 @printf(i8*, ...)

define i32 @main() {
entry:
  %call = call i32 (i8*, ...)* @printf(i8* getelementptr inbounds ([5 x i8]* @main1, i32 0, i32 0))
  ret i32 0
}
